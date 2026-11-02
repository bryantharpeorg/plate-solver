//! Sky-Watcher motor-controller protocol, independent of the serial device.
pub mod codec;
pub mod emulator;
mod protocol;
pub mod serial;
pub mod transport;
pub use protocol::{probe, BoardReport, Firmware, Parameters, ProbeReport, SkyWatcher};

/// A transport failure, malformed message, or motor-controller refusal.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Timeout,
    InvalidReply,
    Refused(u8),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "mount I/O: {e}"),
            Self::Timeout => f.write_str("mount reply timed out"),
            Self::InvalidReply => f.write_str("invalid motor-controller message"),
            Self::Refused(code) => write!(f, "motor-controller refusal !{code:X}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
