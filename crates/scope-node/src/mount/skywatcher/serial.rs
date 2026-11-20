//! Tiny Linux serial shell. All protocol logic operates on the byte-stream port.
use super::transport::BaudStream;
use rustix::{
    fd::OwnedFd,
    fs::{Mode, OFlags},
    termios::{self, ControlModes, OptionalActions, QueueSelector},
};
use std::{io, path::Path, time::Duration};

pub struct Serial {
    fd: OwnedFd,
}
impl Serial {
    /// Open without a controlling terminal; all subsequent I/O is nonblocking.
    pub fn open(device: &Path) -> io::Result<Self> {
        let fd = rustix::fs::open(
            device,
            OFlags::RDWR | OFlags::NOCTTY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let mut serial = Self { fd };
        serial.set_baud(9600)?;
        Ok(serial)
    }
}
impl BaudStream for Serial {
    fn set_baud(&mut self, baud: u32) -> io::Result<()> {
        let mut settings = termios::tcgetattr(&self.fd)?;
        settings.make_raw();
        settings.control_modes.remove(
            ControlModes::PARENB
                | ControlModes::CSTOPB
                | ControlModes::CSIZE
                | ControlModes::CRTSCTS,
        );
        settings
            .control_modes
            .insert(ControlModes::CS8 | ControlModes::CREAD | ControlModes::CLOCAL);
        settings.set_speed(baud)?;
        termios::tcsetattr(&self.fd, OptionalActions::Now, &settings)?;
        termios::tcflush(&self.fd, QueueSelector::IOFlush)?;
        Ok(())
    }
    fn discard_input(&mut self) -> io::Result<Vec<u8>> {
        let mut bytes = [0; 256];
        let mut count = 0;
        while count < bytes.len() {
            match rustix::io::read(&self.fd, &mut bytes[count..]) {
                Ok(0) | Err(rustix::io::Errno::AGAIN) => break,
                Ok(read) => count += read,
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(bytes[..count].to_vec())
    }
    async fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0];
        if retry(|| rustix::io::read(&self.fd, &mut byte)).await? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(byte[0])
    }
    async fn write(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            let count = retry(|| rustix::io::write(&self.fd, bytes)).await?;
            if count == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            bytes = &bytes[count..];
        }
        Ok(())
    }
}

// Nonblocking reads stay cancellation-safe when the probed baud gets no reply.
async fn retry<T>(mut operation: impl FnMut() -> rustix::io::Result<T>) -> io::Result<T> {
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(rustix::io::Errno::INTR) => continue,
            Err(rustix::io::Errno::AGAIN) => tokio::time::sleep(Duration::from_millis(1)).await,
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // US1-S8: exact peer-written bytes are the oracle; no timing tolerance.
    #[tokio::test]
    async fn discard_input_is_nonblocking_bounded_and_leaves_eof_to_read_byte() {
        let (left, right) = std::os::unix::net::UnixStream::pair().unwrap();
        left.set_nonblocking(true).unwrap();
        right.set_nonblocking(true).unwrap();
        let mut serial = Serial { fd: left.into() };
        let mut peer = tokio::net::UnixStream::from_std(right).unwrap();
        peer.write_all(b"=111\r=\r").await.unwrap();
        assert_eq!(serial.discard_input().unwrap(), b"=111\r=\r");
        assert!(serial.discard_input().unwrap().is_empty());
        peer.write_all(&[b'='; 300]).await.unwrap();
        assert_eq!(serial.discard_input().unwrap(), vec![b'='; 256]);
        assert_eq!(serial.discard_input().unwrap(), vec![b'='; 44]);
        peer.write_all(b"=\r").await.unwrap();
        assert_eq!(serial.read_byte().await.unwrap(), b'=');
        assert_eq!(serial.read_byte().await.unwrap(), b'\r');
        drop(peer);
        assert!(serial.discard_input().unwrap().is_empty());
    }

    #[tokio::test]
    async fn nonblocking_bytes_and_disconnect_without_a_serial_device() {
        let (left, right) = std::os::unix::net::UnixStream::pair().unwrap();
        left.set_nonblocking(true).unwrap();
        right.set_nonblocking(true).unwrap();
        let mut serial = Serial { fd: left.into() };
        let mut peer = tokio::net::UnixStream::from_std(right).unwrap();
        let task = tokio::spawn(async move {
            let mut command = [0; 4];
            peer.read_exact(&mut command).await.unwrap();
            assert_eq!(&command, b":e1\r");
            tokio::task::yield_now().await;
            peer.write_all(b"=032890\r").await.unwrap();
        });
        serial.write(b":e1\r").await.unwrap();
        for expected in b"=032890\r" {
            assert_eq!(serial.read_byte().await.unwrap(), *expected);
        }
        task.await.unwrap();
        assert_eq!(
            serial.read_byte().await.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(serial.write(b":e1\r").await.is_err());
    }
}
