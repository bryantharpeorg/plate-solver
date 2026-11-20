//! One outstanding command, echo removal, bounded framing and safe retries.
use super::{codec, Error};
use std::{future::Future, io, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const REPLY_TIMEOUT: Duration = Duration::from_millis(300);

/// A serial byte stream whose baud can change during board discovery.
pub trait BaudStream: Send {
    fn set_baud(&mut self, baud: u32) -> io::Result<()>;
    /// Take unread bytes that have arrived, in order, without waiting.
    fn discard_input(&mut self) -> io::Result<Vec<u8>>;
    fn write(&mut self, bytes: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
    fn read_byte(&mut self) -> impl Future<Output = io::Result<u8>> + Send;
}
impl BaudStream for super::emulator::Emulator {
    fn set_baud(&mut self, baud: u32) -> io::Result<()> {
        self.set_baud(baud)
    }
    fn discard_input(&mut self) -> io::Result<Vec<u8>> {
        Ok(self.discard_input())
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
    /// An unfinished exchange survives cancellation and its own reply timeout.
    open: Option<(tokio::time::Instant, Vec<u8>)>,
    /// This try repeats a frame whose read-out found no reply line.
    again: bool,
    /// The retry read one line, but may still be owed its own reply.
    answered: bool,
    stale_replies: u64,
    last_stale: Option<String>,
}
impl<S: BaudStream> Transport<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            open: None,
            again: false,
            answered: false,
            stale_replies: 0,
            last_stale: None,
        }
    }
    /// Transmissions preceded by stale input, and the last such bytes, escaped.
    pub fn stale(&self) -> (u64, Option<String>) {
        (self.stale_replies, self.last_stale.clone())
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
            self.clear_line(command, axis, &frame).await?;
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
    /// Read out an owed reply to its original deadline, then discard arrived input.
    async fn clear_line(&mut self, command: u8, axis: u8, next: &[u8]) -> Result<(), Error> {
        let mut stale = Vec::new();
        self.again = false;
        if let Some((sent, frame)) = self.open.clone() {
            let deadline = sent + REPLY_TIMEOUT;
            if let Ok(Ok(line)) = tokio::time::timeout_at(deadline, self.line(&frame)).await {
                stale = line;
            } else {
                self.again = !self.answered && frame == next;
            }
            // Leave open in place until the wait ends: clear_line can be cancelled too.
            self.open = None;
            self.answered = false;
        }
        stale.extend(self.stream.discard_input()?);
        if !stale.is_empty() {
            let escaped = stale.escape_ascii().to_string();
            eprintln!(
                "mount stale input before :{}{axis}: {escaped}",
                char::from(command)
            );
            self.stale_replies += 1;
            self.last_stale = Some(escaped);
        }
        Ok(())
    }
    async fn exchange(&mut self, frame: &[u8]) -> Result<Vec<u8>, Error> {
        self.open = Some((tokio::time::Instant::now(), frame.to_vec()));
        self.stream.write(frame).await?;
        let line = self.line(frame).await;
        if self.again && line.is_ok() {
            // FR-016: one more line, once, to this retry's own deadline.
            self.answered = true;
        } else {
            self.open = None;
        }
        self.again = false;
        codec::reply(&line?)
    }
    /// Read a whole line, including its carriage return, skipping line feeds and echo.
    async fn line(&mut self, frame: &[u8]) -> Result<Vec<u8>, Error> {
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
                return Ok(line);
            }
        }
    }
}
impl<S: BaudStream> Transport<S> {
    pub fn set_baud(&mut self, baud: u32) -> Result<(), Error> {
        Ok(self.stream.set_baud(baud)?)
    }
}
