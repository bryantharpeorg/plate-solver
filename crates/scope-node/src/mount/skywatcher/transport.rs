//! One outstanding command, echo removal, bounded framing and safe retries.
use super::{codec, Error};
use std::{future::Future, io, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const REPLY_TIMEOUT: Duration = Duration::from_millis(300);

/// A serial byte stream whose baud can change during board discovery.
pub trait BaudStream: Send {
    fn set_baud(&mut self, baud: u32) -> io::Result<()>;
    fn write(&mut self, bytes: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
    fn read_byte(&mut self) -> impl Future<Output = io::Result<u8>> + Send;
}
impl BaudStream for super::emulator::Emulator {
    fn set_baud(&mut self, baud: u32) -> io::Result<()> {
        self.set_baud(baud)
    }
    async fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.write_all(bytes).await
    }
    async fn read_byte(&mut self) -> io::Result<u8> {
        self.read_u8().await
    }
}

pub struct Transport<S> {
    stream: S,
}
impl<S: BaudStream> Transport<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
    /// `safe` permits at most five transmissions. Relative motion start (`J`)
    /// must never be replayed after a lost acknowledgement.
    pub async fn command(
        &mut self,
        command: u8,
        axis: u8,
        data: &[u8],
        safe: bool,
    ) -> Result<Vec<u8>, Error> {
        let frame = codec::frame(command, axis, data)?;
        let tries = if safe { 5 } else { 1 };
        for attempt in 0..tries {
            let result = tokio::time::timeout(REPLY_TIMEOUT, self.exchange(&frame))
                .await
                .unwrap_or(Err(Error::Timeout));
            match result {
                Err(Error::Timeout | Error::InvalidReply) if attempt + 1 < tries => continue,
                result => return result,
            }
        }
        unreachable!("at least one transmission")
    }
    async fn exchange(&mut self, frame: &[u8]) -> Result<Vec<u8>, Error> {
        self.stream.write(frame).await?;
        let mut line = Vec::new();
        loop {
            let byte = self.stream.read_byte().await?;
            if byte == b'\n' && line.is_empty() {
                continue;
            }
            line.push(byte);
            if line.len() > 16 {
                return Err(Error::InvalidReply);
            }
            if byte == b'\r' {
                if line == frame {
                    line.clear();
                    continue;
                }
                return codec::reply(&line);
            }
        }
    }
}
impl<S: BaudStream> Transport<S> {
    pub fn set_baud(&mut self, baud: u32) -> Result<(), Error> {
        Ok(self.stream.set_baud(baud)?)
    }
}
