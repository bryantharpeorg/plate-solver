#![allow(dead_code)]
//! Shared finder-camera rig. Oracles are hand-written INDI properties and
//! FITS samples; no production parser or gray conversion builds the answers.

use crate::support::{
    def_blob, def_number, def_switch, def_text, set_blob, set_number, set_switch, Scripts,
};
use scope_node::{clock::NodeClock, imaging::indi::IndiCamera};
use std::{sync::Arc, time::Duration};

pub const DEVICE: &str = "Finder CCD";

/// Paused tokio time supplied through the production clock port.
pub struct Clock(pub tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

pub fn clock() -> Arc<dyn NodeClock> {
    Arc::new(Clock(tokio::time::Instant::now()))
}

pub fn reading(ms: u64) -> f64 {
    1000.0 + Duration::from_millis(ms).as_secs_f64()
}

/// A named camera, on its own connection, without neutral controls.
pub fn camera(scripts: &Scripts, device: &str) -> IndiCamera {
    IndiCamera::new(
        scripts.connect(),
        "127.0.0.1:7624".to_string(),
        Some(device.to_string()),
        Vec::new(),
        clock(),
    )
}

/// A mono sensor with the exposure definition's placeholder range replaced
/// by the first answer's range, as the bench device program reports it.
pub fn finder(gain: f64) -> String {
    let mut script = format!(
        "@ device {DEVICE}\n@ expect getProperties @version=1.7\n@ send\n{}\n{}\n\
         @ expect newSwitchVector CONNECTION CONNECT=On\n@ send\n{}\n",
        def_switch(
            DEVICE,
            "CONNECTION",
            &[("CONNECT", "Off"), ("DISCONNECT", "On")]
        ),
        def_text(DEVICE, "DRIVER_INFO", &[("DRIVER_INTERFACE", "2")]),
        set_switch(DEVICE, "CONNECTION", "Ok", None, &[("CONNECT", "On")]),
    );
    for line in [
        def_number(
            DEVICE,
            "CCD_EXPOSURE",
            &[("CCD_EXPOSURE_VALUE", 1.0, 0.0, 3600.0)],
        ),
        def_switch(DEVICE, "CCD_ABORT_EXPOSURE", &[("ABORT", "Off")]),
        def_number(
            DEVICE,
            "CCD_INFO",
            &[
                ("CCD_MAX_X", 1280.0, 1280.0, 1280.0),
                ("CCD_MAX_Y", 960.0, 960.0, 960.0),
                ("CCD_PIXEL_SIZE", 3.75, 3.75, 3.75),
                ("CCD_BITSPERPIXEL", 16.0, 16.0, 16.0),
            ],
        ),
        def_number(
            DEVICE,
            "CCD_CONTROLS",
            &[("Gain", gain, 0.0, 100.0), ("Offset", 10.0, 0.0, 100.0)],
        ),
        def_switch(
            DEVICE,
            "CCD_FRAME_TYPE",
            &[
                ("FRAME_LIGHT", "On"),
                ("FRAME_DARK", "Off"),
                ("FRAME_FLAT", "Off"),
                ("FRAME_BIAS", "Off"),
            ],
        ),
        def_blob(DEVICE, "CCD1", "CCD1"),
    ] {
        script.push_str(&line);
        script.push('\n');
    }
    script.push_str(&format!(
        "<setNumberVector device=\"{DEVICE}\" name=\"CCD_EXPOSURE\" state=\"Ok\"><oneNumber name=\"CCD_EXPOSURE_VALUE\" min=\"3.2e-05\" max=\"2000\">1</oneNumber></setNumberVector>\n\
         @ expect after 200 enableBLOB text=Also\n"
    ));
    script
}

/// Await the camera's gain answer before arming the exposure; Offset is
/// sent back unchanged, since CCD_CONTROLS is one vector.
pub fn gain_step(gain: f64) -> String {
    format!(
        "@ expect newNumberVector CCD_CONTROLS Gain={gain} Offset=10\n@ send after 100\n{}\n",
        set_number(
            DEVICE,
            "CCD_CONTROLS",
            "Ok",
            None,
            &[("Gain", gain), ("Offset", 10.0)]
        ),
    )
}

/// The next exposure's two requests, also used before a connection closes.
pub fn light_request(seconds: f64) -> String {
    format!(
        "@ expect newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
         @ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE={seconds}\n"
    )
}

/// Busy 100 ms after the request, a FITS BLOB one exposure later.
pub fn answers(device: &str, seconds: f64, image: &[u8]) -> String {
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    let encoded = BASE64_STANDARD.encode(image);
    format!(
        "@ send after 100\n{}\n@ send after {}\n{}\n@ send\n{}\n",
        set_number(
            device,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", seconds)]
        ),
        (seconds * 1000.0) as u64,
        set_blob(device, "CCD1", "Ok", None, &[("CCD1", ".fits", &encoded)]),
        set_number(
            device,
            "CCD_EXPOSURE",
            "Ok",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.0)]
        ),
    )
}

pub fn exposure(seconds: f64, image: &[u8]) -> String {
    light_request(seconds) + &answers(DEVICE, seconds, image)
}

/// The tests' own 80-byte FITS card writer, independent of fits::read.
fn card(keyword: &str, value: &str) -> [u8; 80] {
    let mut out = [b' '; 80];
    let text = format!("{keyword:<8}= {value:>20}");
    out[..text.len()].copy_from_slice(text.as_bytes());
    out
}

/// Whole FITS blocks, with an END card and zero-padded data.
pub fn fits(width: u32, height: u32, bits: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in [
        card("SIMPLE", "T"),
        card("BITPIX", &bits.to_string()),
        card("NAXIS", "2"),
        card("NAXIS1", &width.to_string()),
        card("NAXIS2", &height.to_string()),
    ] {
        out.extend_from_slice(&line);
    }
    if bits == 16 {
        out.extend_from_slice(&card("BZERO", "32768"));
        out.extend_from_slice(&card("BSCALE", "1"));
    }
    let mut end = [b' '; 80];
    end[..3].copy_from_slice(b"END");
    out.extend_from_slice(&end);
    out.resize(out.len().next_multiple_of(2880), b' ');
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(2880), 0);
    out
}

/// Unsigned samples 0, 1, 255, 256, 0x1234, 0x8000, 0xff00, 0xffff,
/// stored as signed big-endian FITS values with BZERO 32768.
pub fn worked_frame() -> Vec<u8> {
    fits(
        4,
        2,
        16,
        &[
            0x80, 0x00, 0x80, 0x01, 0x80, 0xFF, 0x81, 0x00, 0x92, 0x34, 0x00, 0x00, 0x7F, 0x00,
            0x7F, 0xFF,
        ],
    )
}

pub fn ramp() -> Vec<u8> {
    fits(3, 2, 8, &[0, 1, 2, 253, 254, 255])
}

pub fn flat(value: u8) -> Vec<u8> {
    fits(3, 2, 8, &[value; 6])
}
