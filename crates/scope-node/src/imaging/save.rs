//! The node's own copy of each frame (030 US7): one FITS file under the
//! folder the `[imaging]` table names, written before the frame's header
//! leaves the node. The format is the one the console's own writer follows
//! (031's plan.md § FITS at the console), written out here because the
//! scope node must not depend on the console's crate.

use crate::imaging::ImagingFrame;
use std::{io, io::Write, path::Path};

/// One card's value.
enum Value<'a> {
    Logic(bool),
    Int(i64),
    Real(f64),
    Text(&'a str),
}

/// Write one frame as FITS under `dir` and answer the file's name. The
/// folder is made when it is missing; the name is `frame-<id, six
/// digits>.fits`, then `frame-<id>-2.fits`, `-3` and so on while the name is
/// taken; the bytes land under a `.part` name and are renamed into place.
pub fn write(
    dir: &Path,
    id: u64,
    frame: &ImagingFrame,
    model: &str,
    purpose: &str,
) -> io::Result<String> {
    std::fs::create_dir_all(dir)?;
    let name = unused_name(dir, id)?;
    let part = dir.join(format!("{name}.part"));
    let mut file = std::fs::File::create_new(&part)?;
    file.write_all(&file_bytes(frame, id, model, purpose))?;
    drop(file);
    std::fs::rename(&part, dir.join(&name))?;
    Ok(name)
}

/// The first of `frame-<id, six digits>.fits`, `frame-<id, six digits>-2.fits`,
/// `-3` and so on that is not taken.
fn unused_name(dir: &Path, id: u64) -> io::Result<String> {
    let mut suffix = 1;
    loop {
        let name = if suffix == 1 {
            format!("frame-{id:06}.fits")
        } else {
            format!("frame-{id:06}-{suffix}.fits")
        };
        if !dir.join(&name).try_exists()? {
            return Ok(name);
        }
        suffix += 1;
    }
}

/// The whole file: the cards of plan.md § The node's copy, the header
/// padded with spaces to a multiple of 2880 bytes, then the samples, padded
/// with zeros to the same.
fn file_bytes(frame: &ImagingFrame, id: u64, model: &str, purpose: &str) -> Vec<u8> {
    // The origin is written in binned pixels, so the binning is never zero
    // below even when a camera answers a frame without one.
    let binning = frame.binning.max(1);
    let mut cards = vec![
        card("SIMPLE", Value::Logic(true)),
        card("BITPIX", Value::Int(i64::from(frame.bits))),
        card("NAXIS", Value::Int(2)),
        card("NAXIS1", Value::Int(i64::from(frame.width))),
        card("NAXIS2", Value::Int(i64::from(frame.height))),
    ];
    if frame.bits == 16 {
        cards.push(card("BZERO", Value::Int(32_768)));
        cards.push(card("BSCALE", Value::Int(1)));
    }
    cards.push(card("EXPTIME", Value::Real(frame.exposure_ms / 1000.0)));
    if let Some(gain) = frame.gain {
        cards.push(card("GAIN", Value::Real(gain)));
    }
    if let Some(offset) = frame.offset {
        cards.push(card("OFFSET", Value::Real(offset)));
    }
    cards.push(card("XBINNING", Value::Int(i64::from(frame.binning))));
    cards.push(card("YBINNING", Value::Int(i64::from(frame.binning))));
    cards.push(card(
        "XORGSUBF",
        Value::Int(i64::from(frame.roi.x / binning)),
    ));
    cards.push(card(
        "YORGSUBF",
        Value::Int(i64::from(frame.roi.y / binning)),
    ));
    if let Some(temp) = frame.temp_c {
        cards.push(card("CCD-TEMP", Value::Real(temp)));
    }
    if let Some(bayer) = frame.bayer {
        cards.push(card("BAYERPAT", Value::Text(bayer.as_str())));
    }
    cards.push(card("INSTRUME", Value::Text(model)));
    cards.push(card("IMAGETYP", Value::Text(image_type(purpose))));
    cards.push(card("ROWORDER", Value::Text("TOP-DOWN")));
    cards.push(card("FRAMEID", Value::Int(id as i64)));
    cards.push(card("PURPOSE", Value::Text(purpose)));
    // The node has no wall clock, so there is no DATE-OBS; the exposure's
    // start on its own clock stands in (FR-035).
    cards.push(card("NODE-T0", Value::Real(frame.start_s)));
    cards.push(end_card());
    let mut out = cards.concat();
    while out.len() % 2880 != 0 {
        out.push(b' ');
    }
    out.extend(stored_samples(frame));
    while out.len() % 2880 != 0 {
        out.push(0);
    }
    out
}

/// The frame's samples as the file stores them: a 16-bit sample as the
/// big-endian signed value `sample − 32768`, an 8-bit one as it is.
fn stored_samples(frame: &ImagingFrame) -> Vec<u8> {
    match frame.bits {
        16 => frame
            .samples
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pair| {
                let sample = u16::from_le_bytes(*pair);
                ((i32::from(sample) - 32_768) as i16).to_be_bytes()
            })
            .collect(),
        _ => frame.samples.clone(),
    }
}

/// `DARK` for the purpose `dark`, `FLAT` for `flat`, `LIGHT` otherwise.
fn image_type(purpose: &str) -> &'static str {
    match purpose {
        "dark" => "DARK",
        "flat" => "FLAT",
        _ => "LIGHT",
    }
}

/// One 80-byte card: the keyword padded to 8, `= `, then a logical or a
/// number right-justified to column 30, or a text in single quotes from
/// column 11, padded inside the quotes to 8 characters, quotes doubled and
/// cut at 68.
fn card(keyword: &str, value: Value) -> [u8; 80] {
    debug_assert!(
        keyword.len() <= 8,
        "the {keyword} card is too long for FITS"
    );
    let mut out = [b' '; 80];
    out[..keyword.len()].copy_from_slice(keyword.as_bytes());
    out[8] = b'=';
    out[9] = b' ';
    let value = match value {
        Value::Logic(true) => "T".to_string(),
        Value::Logic(false) => "F".to_string(),
        Value::Int(number) => number.to_string(),
        Value::Real(number) => format!("{number:?}").replace('e', "E"),
        Value::Text(text) => {
            let mut inner = text.replace('\'', "''");
            while inner.chars().count() < 8 {
                inner.push(' ');
            }
            let inner: String = inner.chars().take(68).collect();
            format!("'{inner}'")
        }
    };
    let start = if value.starts_with('\'') {
        10
    } else {
        // A logical or a number ends in column 30, however long it is.
        (30 - value.len()).max(10)
    };
    out[start..start + value.len()].copy_from_slice(value.as_bytes());
    out
}

/// The `END` card: the keyword alone in its eight columns.
fn end_card() -> [u8; 80] {
    let mut out = [b' '; 80];
    out[..3].copy_from_slice(b"END");
    out
}
