//! The bring-up probe (US8): describe the camera, take three exposures and
//! write one line for each fact, so the observer learns the download time
//! before the console is involved.
use crate::{
    clock::NodeClock,
    imaging::{Exposure, ImagingCamera, ImagingError},
};
use std::io::Write;

/// One line, ended by a newline and flushed as it is written. A failed write
/// answers `Failed`.
fn line(out: &mut (dyn Write + Send), text: &str) -> Result<(), ImagingError> {
    writeln!(out, "{text}")
        .and_then(|()| out.flush())
        .map_err(|error| ImagingError::Failed(format!("cannot write the probe's output: {error}")))
}

/// Describe the camera, take three exposures, and write one line for each
/// fact. `exposure_ms` is each exposure's length and each exposure's purpose
/// is `probe`. The lines are those of US8-S1, numbers written with `Display`;
/// a camera error ends the run with that error, and a failed write answers
/// `Failed`.
pub async fn run(
    camera: &dyn ImagingCamera,
    clock: &dyn NodeClock,
    exposure_ms: f64,
    out: &mut (dyn Write + Send),
) -> Result<(), ImagingError> {
    let info = camera.describe().await?;
    line(out, &format!("imaging camera: {}", info.model))?;
    match &info.serial {
        Some(serial) => line(out, &format!("serial: {serial}"))?,
        None => line(out, "serial: none")?,
    }
    line(
        out,
        &format!(
            "sensor: {} x {} px, {} um pixels, {} bits, {}",
            info.width,
            info.height,
            info.pixel_um,
            info.bits,
            info.bayer.map(|bayer| bayer.as_str()).unwrap_or("mono")
        ),
    )?;
    line(
        out,
        &format!(
            "exposure: {} to {} ms",
            info.exposure_ms.min, info.exposure_ms.max
        ),
    )?;
    for (name, range) in [("gain", info.gain), ("offset", info.offset)] {
        match range {
            Some(range) => line(out, &format!("{name}: {} to {}", range.min, range.max))?,
            None => line(out, &format!("{name}: none"))?,
        }
    }
    let binnings = info
        .binnings
        .iter()
        .map(|binning| binning.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    line(out, &format!("binnings: {binnings}"))?;
    line(
        out,
        if info.cooler {
            "cooler: yes"
        } else {
            "cooler: no"
        },
    )?;
    for n in 1..=3 {
        let frame = camera
            .expose(&Exposure {
                exposure_ms,
                purpose: "probe".into(),
            })
            .await?;
        // How long after the end of the exposure the frame was in hand.
        let in_hand_ms = ((clock.now() - frame.end_s) * 1000.0).round().max(0.0);
        line(
            out,
            &format!(
                "exposure {n}: {} x {}, {} bytes, in hand {} ms after the end of the exposure",
                frame.width,
                frame.height,
                frame.samples.len(),
                in_hand_ms
            ),
        )?;
    }
    Ok(())
}
