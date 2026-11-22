//! YUV420 framing at the row stride the ISP writes; discard Y padding and UV.
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt};

/// The Raspberry Pi ISP behind the camera, which sets the row stride of
/// rpicam-vid's YUV420 frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Isp {
    /// VC4, as on the Pi 3B+: rows padded to a multiple of 64 bytes (measured).
    Vc4,
    /// PiSP, as on the Pi 5: rows padded to a multiple of 128 bytes (measured).
    Pisp,
}
impl Isp {
    /// The row stride of a frame `width` pixels wide.
    pub fn stride(self, width: u32) -> usize {
        let alignment = match self {
            Self::Vc4 => 64,
            Self::Pisp => 128,
        };
        (width as usize).div_ceil(alignment) * alignment
    }
}

/// The VC4's row stride, measured on the Pi 3B+.
pub fn stride(width: u32) -> usize {
    Isp::Vc4.stride(width)
}

pub struct YuvReader<R> {
    reader: R,
    width: usize,
    height: usize,
    stride: usize,
    bytes: Vec<u8>,
    frames: u64,
}
impl<R: AsyncRead + Unpin> YuvReader<R> {
    pub fn new(reader: R, width: u32, height: u32, stride: usize) -> io::Result<Self> {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "YUV420 requires positive even width/height and an even stride at least the width",
            )
        };
        if width == 0
            || height == 0
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
            || stride < width as usize
            || !stride.is_multiple_of(2)
        {
            return Err(invalid());
        }
        let size = stride
            .checked_mul(height as usize)
            .and_then(|y| y.checked_mul(3))
            .map(|size| size / 2)
            .ok_or_else(invalid)?;
        Ok(Self {
            reader,
            width: width as usize,
            height: height as usize,
            stride,
            bytes: vec![0; size],
            frames: 0,
        })
    }
    pub async fn next(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.reader.read(&mut self.bytes[..1]).await? == 0 {
            return Ok(None);
        }
        self.reader.read_exact(&mut self.bytes[1..]).await?;
        self.frames += 1;
        // rpicam-vid writes 0 in every row's padding, luma and chroma (measured on
        // both ISPs). Anything else there means the frame is not at this stride.
        let (width, stride, luma) = (self.width, self.stride, self.stride * self.height);
        let mut padding = self.bytes[..luma]
            .chunks_exact(stride)
            .map(|row| &row[width..])
            .chain(
                self.bytes[luma..]
                    .chunks_exact(stride / 2)
                    .map(|row| &row[width / 2..]),
            );
        if padding.any(|bytes| bytes.iter().any(|&b| b != 0)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "frame {} does not fit a {stride}-byte row stride at width {width}: \
                     its row padding is not 0",
                    self.frames
                ),
            ));
        }
        Ok(Some(
            self.bytes[..self.stride * self.height]
                .chunks_exact(self.stride)
                .flat_map(|row| row[..self.width].iter().copied())
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn invalid_geometry_and_partial_frames_are_errors() {
        for (w, h, s) in [(0, 2, 64), (3, 2, 64), (2, 3, 64), (4, 2, 2), (2, 2, 3)] {
            assert!(YuvReader::new(&b""[..], w, h, s).is_err());
        }
        let mut reader = YuvReader::new(&b"short"[..], 2, 2, 64).unwrap();
        assert_eq!(
            reader.next().await.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
}
