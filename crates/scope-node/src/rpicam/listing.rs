//! Parse rpicam-hello's saved listing; crop sizes are not sensor modes.
use super::yuv::Isp;
use crate::{camera::Mode, CameraError, CameraInfo};

fn is_camera(line: &str) -> bool {
    line.split_once(" : ")
        .is_some_and(|(index, _)| index.parse::<usize>().is_ok())
}

/// The ISP behind the listing's first camera. The listing's `PipelineHandler`
/// names it; a listing without one, from an rpicam-apps that did not print it,
/// is placed by the sensor's device path, which on a Pi 5 runs through its RP1.
pub fn isp(listing: &str) -> Result<Isp, CameraError> {
    let mut lines = listing.lines().map(str::trim).skip_while(|l| !is_camera(l));
    let camera = lines
        .next()
        .ok_or_else(|| CameraError("camera listing contains no cameras or modes".into()))?;
    for line in lines.take_while(|l| !is_camera(l)) {
        if let Some(("PipelineHandler", handler)) = line.split_once(" : ") {
            return match handler.trim() {
                "rpi/vc4" => Ok(Isp::Vc4),
                "rpi/pisp" => Ok(Isp::Pisp),
                other => Err(CameraError(format!(
                    "camera listing names the pipeline handler {other}, whose row stride the node does not know"
                ))),
            };
        }
    }
    Ok(if camera.contains("/rp1/") {
        Isp::Pisp
    } else {
        Isp::Vc4
    })
}

fn size(text: &str) -> Option<(u32, u32)> {
    let (width, rest) = text.split_once('x')?;
    let height: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let (width, height) = (width.parse().ok()?, height.parse().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

/// Parse the verbose listing, including the largest mode's control ranges.
pub fn parse(listing: &str) -> Result<Vec<CameraInfo>, CameraError> {
    let mut cameras: Vec<CameraInfo> = Vec::new();
    for line in listing.lines() {
        let line = line.trim();
        if let Some((index, rest)) = line.split_once(" : ") {
            if index.parse::<usize>().is_ok() {
                let (model, description) = rest
                    .split_once(" [")
                    .ok_or_else(|| CameraError("camera listing has no sensor dimensions".into()))?;
                let (width, height) = size(description).ok_or_else(|| {
                    CameraError("invalid sensor dimensions in camera listing".into())
                })?;
                cameras.push(CameraInfo {
                    model: model.into(),
                    width,
                    height,
                    ..CameraInfo::default()
                });
                continue;
            }
        }
        let Some(camera) = cameras.last_mut() else {
            continue;
        };
        for (name, min, max, scale) in [
            (
                "ExposureTime",
                &mut camera.exposure_min_ms,
                &mut camera.exposure_max_ms,
                1000.0,
            ),
            (
                "AnalogueGain",
                &mut camera.gain_min,
                &mut camera.gain_max,
                1.0,
            ),
        ] {
            if let Some((key, value)) = line.split_once(':') {
                if key.trim() == name {
                    let range = value
                        .trim()
                        .strip_prefix('[')
                        .and_then(|v| v.strip_suffix(']'))
                        .and_then(|v| v.split_once(".."));
                    let values = range
                        .and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?)));
                    let (a, b) = values
                        .filter(|(a, b)| a.is_finite() && b.is_finite() && *a > 0.0 && b >= a)
                        .ok_or_else(|| {
                            CameraError(format!("invalid {name} range in camera listing"))
                        })?;
                    *min = a / scale;
                    *max = b / scale;
                }
            }
        }
        let mode_text = if line.starts_with("Modes:") {
            line.split_once(" : ").map(|(_, mode)| mode).unwrap_or("")
        } else {
            line
        };
        let Some((width, height)) = size(mode_text.split_whitespace().next().unwrap_or("")) else {
            continue;
        };
        let fps = mode_text
            .split_once('[')
            .and_then(|(_, rest)| rest.split_whitespace().next())
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| CameraError("invalid mode frame rate in camera listing".into()))?;
        if !camera
            .modes
            .iter()
            .any(|m| m.width == width && m.height == height)
        {
            camera.modes.push(Mode { width, height, fps });
        }
    }
    if cameras.is_empty() || cameras.iter().any(|camera| camera.modes.is_empty()) {
        return Err(CameraError(
            "camera listing contains no cameras or modes".into(),
        ));
    }
    if cameras
        .iter()
        .any(|camera| camera.exposure_min_ms <= 0.0 || camera.gain_min <= 0.0)
    {
        return Err(CameraError("verbose camera listing is missing ExposureTime or AnalogueGain ranges; use --list-cameras -v".into()));
    }
    Ok(cameras)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_camera_and_malformed_sensor_are_errors() {
        assert!(parse("No cameras available!").is_err());
        assert!(parse("0 : broken [wrong]").is_err());
        assert!(parse("0 : sensor [640x480 8-bit MONO]").is_err());
    }
}
