//! Read each object as it arrives, without waiting for the live array's closing `]`.
use crate::{CameraError, GrayFrame};
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

#[derive(Debug, Deserialize)]
pub struct Metadata {
    #[serde(rename = "SensorTimestamp")]
    pub sensor_timestamp_ns: u64,
    #[serde(rename = "ExposureTime")]
    pub exposure_time_us: u64,
    #[serde(rename = "AnalogueGain")]
    pub analogue_gain: f64,
    #[serde(rename = "DigitalGain", default = "one")]
    pub digital_gain: f64,
}
fn one() -> f64 {
    1.0
}
impl Metadata {
    pub fn frame(self, pixels: Vec<u8>, width: u32, height: u32) -> GrayFrame {
        let start_s = self.sensor_timestamp_ns as f64 / 1e9;
        GrayFrame {
            pixels,
            width,
            height,
            start_s,
            end_s: start_s + self.exposure_time_us as f64 / 1e6,
            exposure_ms: self.exposure_time_us as f64 / 1000.0,
            gain: self.analogue_gain * self.digital_gain,
        }
    }
}

pub struct MetadataReader<R> {
    reader: BufReader<R>,
    started: bool,
    first: bool,
    ended: bool,
    closing: bool,
}
impl<R: AsyncRead + Unpin> MetadataReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            started: false,
            first: true,
            ended: false,
            closing: false,
        }
    }
    async fn byte(&mut self) -> Result<u8, CameraError> {
        Ok(self.reader.read_u8().await?)
    }
    async fn nonspace(&mut self) -> Result<u8, CameraError> {
        loop {
            let byte = self.byte().await?;
            if !byte.is_ascii_whitespace() {
                return Ok(byte);
            }
        }
    }
    pub async fn next(&mut self) -> Result<Option<Metadata>, CameraError> {
        if self.ended {
            return Ok(None);
        }
        if !self.started {
            if self.nonspace().await? != b'[' {
                return Err(CameraError(
                    "rpicam metadata must start with a JSON array".into(),
                ));
            }
            self.started = true;
        }
        if self.closing {
            if self.nonspace().await? != b'}' {
                return Err(CameraError("rpicam metadata is missing a brace".into()));
            }
            self.closing = false;
        }
        let mut byte = self.nonspace().await?;
        if byte == b']' {
            self.ended = true;
            return Ok(None);
        }
        if !self.first {
            if byte != b',' {
                return Err(CameraError("rpicam metadata is missing a comma".into()));
            }
            byte = self.nonspace().await?;
        }
        if byte != b'{' {
            return Err(CameraError("rpicam metadata expected an object".into()));
        }
        let mut object = vec![byte];
        let (mut depth, mut quoted, mut escaped, mut last) = (1usize, false, false, byte);
        while depth != 0 {
            byte = self.byte().await?;
            // rpicam flushes each line but holds the closing brace until the next
            // frame's metadata, written after that frame's pixels. Waiting for the
            // brace would stop the pixel reads, so the last line ends the object.
            if byte == b'\n' && depth == 1 && !quoted && last != b',' && last != b'{' {
                object.push(b'}');
                self.closing = true;
                break;
            }
            if !byte.is_ascii_whitespace() {
                last = byte;
            }
            object.push(byte);
            if object.len() > 1024 * 1024 {
                return Err(CameraError("rpicam metadata object exceeds 1 MiB".into()));
            }
            if quoted {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    quoted = false;
                }
            } else {
                match byte {
                    b'"' => quoted = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => depth -= 1,
                    _ => {}
                }
            }
        }
        let metadata: Metadata = serde_json::from_slice(&object)
            .map_err(|e| CameraError(format!("rpicam metadata: {e}")))?;
        if metadata.exposure_time_us == 0
            || !metadata.analogue_gain.is_finite()
            || metadata.analogue_gain <= 0.0
            || !metadata.digital_gain.is_finite()
            || metadata.digital_gain <= 0.0
        {
            return Err(CameraError(
                "rpicam metadata has invalid ExposureTime or gain".into(),
            ));
        }
        self.first = false;
        Ok(Some(metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn first_object_arrives_before_closing_array_or_eof() {
        use tokio::io::AsyncWriteExt;
        let (mut writer, reader) = tokio::io::duplex(512);
        writer.write_all(br#"[{"SensorTimestamp":1000000000,"ExposureTime":20000,"AnalogueGain":2,"extra":{"quoted":"}\\\"{"}}"#).await.unwrap();
        let metadata = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            MetadataReader::new(reader).next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        assert_eq!(metadata.sensor_timestamp_ns, 1_000_000_000);
    }
    #[tokio::test]
    async fn object_ends_on_its_last_line_before_the_brace_is_flushed() {
        use tokio::io::AsyncWriteExt;
        let object = |t: u32| {
            format!(
                "{{\n    \"ExposureTime\": 1000,\n    \"ColourGains\": [ 1.5, 1.2 ],\n    \"SensorTimestamp\": {t},\n    \"AnalogueGain\": 8.000000\n"
            )
        };
        let wait = std::time::Duration::from_millis(100);
        let (mut writer, reader) = tokio::io::duplex(512);
        let mut metadata = MetadataReader::new(reader);
        writer
            .write_all(format!("[\n{}", object(1)).as_bytes())
            .await
            .unwrap();
        let first = tokio::time::timeout(wait, metadata.next()).await.unwrap();
        assert_eq!(first.unwrap().unwrap().sensor_timestamp_ns, 1);
        writer
            .write_all(format!("}},\n{}", object(2)).as_bytes())
            .await
            .unwrap();
        let second = tokio::time::timeout(wait, metadata.next()).await.unwrap();
        assert_eq!(second.unwrap().unwrap().sensor_timestamp_ns, 2);
        writer.write_all(b"}\n]\n").await.unwrap();
        assert!(metadata.next().await.unwrap().is_none());
    }
    #[tokio::test]
    async fn malformed_or_missing_metadata_is_not_a_frame() {
        for input in [
            "{}",
            "[{}",
            "[{}]",
            "[42]",
            "[{\"SensorTimestamp\":1,\"ExposureTime\":0,\"AnalogueGain\":1}]",
        ] {
            assert!(
                MetadataReader::new(input.as_bytes()).next().await.is_err(),
                "{input}"
            );
        }
    }
}
