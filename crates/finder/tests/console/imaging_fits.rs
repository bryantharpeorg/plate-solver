//! Reading a saved imaging frame back (031 US3): the bytes `encode` writes, read
//! with this test's own reader, the worked FITS frame of plan.md § FITS at the
//! console read back by `decode`, and every refusal with its text. Oracles: the
//! worked bytes, cards and texts of US3-S1, worked here and never read through
//! the console's FITS code.

use finder::imaging::fits::{decode, encode, CardValue, FitsImage};
use serde_json::{json, Value};

/// The samples of the worked images as `FitsImage` keeps them: little-endian.
fn le(samples: &[u16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

/// The worked 4 × 2, 16-bit image of US3-S1, with the five cards after the
/// structural ones.
fn worked_image() -> FitsImage {
    FitsImage {
        width: 4,
        height: 2,
        bits: 16,
        samples: le(&[0, 1, 2, 3, 32768, 40000, 65534, 65535]),
        cards: vec![
            ("EXPTIME".to_string(), CardValue::Real(0.5)),
            ("GAIN".to_string(), CardValue::Real(100.0)),
            ("XBINNING".to_string(), CardValue::Int(1)),
            ("BAYERPAT".to_string(), CardValue::Text("RGGB".to_string())),
            ("IMAGETYP".to_string(), CardValue::Text("LIGHT".to_string())),
        ],
    }
}

/// A card's number, compared as a number and not as its text (the shared setup).
fn number(text: &str) -> Value {
    if text.contains('.') || text.contains('E') || text.contains('e') {
        let value: f64 = text.parse().expect("a number card");
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .expect("a finite real")
    } else {
        Value::Number(text.parse::<i64>().expect("a whole number").into())
    }
}

/// FITS bytes read with this test's own code, never through the console's (the
/// shared setup): the 80-byte cards up to `END`, the header padded to 2880, then
/// the data as it is stored.
struct Read {
    cards: Vec<(String, Value)>,
    data: Vec<u8>,
}

fn read(bytes: &[u8]) -> Read {
    let mut cards = Vec::new();
    let mut offset = 0;
    loop {
        let card = &bytes[offset..offset + 80];
        let keyword = String::from_utf8_lossy(&card[..8]).trim_end().to_string();
        if keyword == "END" {
            cards.push((keyword, Value::Null));
            offset += 80;
            break;
        }
        assert_eq!(
            &card[8..10],
            b"= ",
            "the {keyword} card is not a value card"
        );
        let value = String::from_utf8_lossy(&card[10..]).trim().to_string();
        let value = if let Some(text) = value.strip_prefix('\'') {
            Value::String(
                text.strip_suffix('\'')
                    .expect("a closing quote")
                    .replace("''", "'")
                    .trim_end()
                    .to_string(),
            )
        } else if value == "T" {
            Value::Bool(true)
        } else if value == "F" {
            Value::Bool(false)
        } else {
            number(&value)
        };
        cards.push((keyword, value));
        offset += 80;
    }
    Read {
        cards,
        data: bytes[offset.div_ceil(2880) * 2880..].to_vec(),
    }
}

/// One 80-byte card of the worked frame: the keyword padded to 8, `= ` in
/// columns 9 and 10, then a quoted text from column 11 or a number
/// right-justified to column 30 (plan.md § FITS at the console).
fn card(keyword: &str, value: &str) -> [u8; 80] {
    let mut card = [b' '; 80];
    card[..keyword.len()].copy_from_slice(keyword.as_bytes());
    card[8] = b'=';
    card[9] = b' ';
    let bytes = value.as_bytes();
    if bytes[0] == b'\'' {
        card[10..10 + bytes.len()].copy_from_slice(bytes);
    } else {
        card[30 - bytes.len()..30].copy_from_slice(bytes);
    }
    card
}

/// The worked FITS frame of plan.md § FITS at the console, from the cards and
/// bytes given there: one header block, then the stored samples and zeros to
/// 2880. `replacements` swaps a card's value where a refusal asks for it.
fn frame_of(cards: &[(&'static str, &'static str)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (keyword, value) in cards {
        bytes.extend_from_slice(&card(keyword, value));
    }
    let mut end = [b' '; 80];
    end[..3].copy_from_slice(b"END");
    bytes.extend_from_slice(&end);
    bytes.resize(2880, b' ');
    bytes.extend_from_slice(&[
        0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE, 0x7F,
        0xFF,
    ]);
    bytes.resize(5760, 0);
    bytes
}

/// The worked frame's cards, with the values of `replacements` in theirs.
fn worked_cards(
    replacements: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let mut cards = vec![
        ("SIMPLE", "T"),
        ("BITPIX", "16"),
        ("NAXIS", "2"),
        ("NAXIS1", "4"),
        ("NAXIS2", "2"),
        ("BZERO", "32768"),
        ("BSCALE", "1"),
        ("XBINNING", "1"),
        ("YBINNING", "1"),
        ("CCD-TEMP", "-9.5"),
        ("BAYERPAT", "'RGGB    '"),
    ];
    for (keyword, value) in replacements {
        cards
            .iter_mut()
            .find(|(existing, _)| existing == keyword)
            .expect("a card of the worked frame")
            .1 = *value;
    }
    cards
}

/// `decode`'s error text for bytes it refuses.
fn refusal(bytes: &[u8]) -> String {
    decode(bytes).unwrap_err().to_string()
}

/// US3-S1: `encode` writes the worked 16-bit image — 5760 bytes, the cards in
/// order, a whole real right-justified to column 30, each sample less 32768 as a
/// big-endian signed number — and `decode` reads it back.
#[test]
fn encode_writes_the_worked_16_bit_image_and_decode_reads_it_back() {
    let image = worked_image();
    let bytes = encode(&image);
    assert_eq!(bytes.len(), 5760);
    let read = read(&bytes);
    assert_eq!(
        read.cards,
        vec![
            ("SIMPLE".to_string(), json!(true)),
            ("BITPIX".to_string(), json!(16)),
            ("NAXIS".to_string(), json!(2)),
            ("NAXIS1".to_string(), json!(4)),
            ("NAXIS2".to_string(), json!(2)),
            ("BZERO".to_string(), json!(32768)),
            ("BSCALE".to_string(), json!(1)),
            ("EXPTIME".to_string(), json!(0.5)),
            ("GAIN".to_string(), json!(100.0)),
            ("XBINNING".to_string(), json!(1)),
            ("BAYERPAT".to_string(), json!("RGGB")),
            ("IMAGETYP".to_string(), json!("LIGHT")),
            ("END".to_string(), Value::Null),
        ]
    );
    let gain = read
        .cards
        .iter()
        .position(|(keyword, _)| keyword == "GAIN")
        .unwrap();
    let gain = std::str::from_utf8(&bytes[gain * 80..gain * 80 + 80]).unwrap();
    assert!(
        gain.starts_with("GAIN    =                100.0"),
        "the GAIN card is {gain:?}"
    );
    assert_eq!(
        &read.data[..16],
        &[
            0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE,
            0x7F, 0xFF
        ]
    );
    let back = decode(&bytes).unwrap();
    assert_eq!(back, image);
    assert_eq!(
        back.cards
            .iter()
            .find(|(keyword, _)| keyword == "GAIN")
            .unwrap()
            .1,
        CardValue::Real(100.0)
    );
}

/// US3-S1: `encode` writes an 8-bit image with no `BZERO` and no `BSCALE`, and
/// `decode` reads it back.
#[test]
fn encode_writes_an_8_bit_image_and_decode_reads_it_back() {
    let image = FitsImage {
        width: 3,
        height: 2,
        bits: 8,
        samples: vec![0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF],
        cards: Vec::new(),
    };
    let bytes = encode(&image);
    assert_eq!(bytes.len(), 5760);
    let read = read(&bytes);
    assert_eq!(
        read.cards,
        vec![
            ("SIMPLE".to_string(), json!(true)),
            ("BITPIX".to_string(), json!(8)),
            ("NAXIS".to_string(), json!(2)),
            ("NAXIS1".to_string(), json!(3)),
            ("NAXIS2".to_string(), json!(2)),
            ("END".to_string(), Value::Null),
        ]
    );
    assert_eq!(&read.data[..6], &[0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF]);
    assert_eq!(decode(&bytes).unwrap(), image);
}

/// US3-S1: `decode` reads the worked FITS frame of plan.md § FITS at the
/// console, built by this test from the cards and bytes given there.
#[test]
fn decode_reads_the_worked_fits_frame() {
    let image = decode(&frame_of(&worked_cards(&[]))).unwrap();
    assert_eq!(image.width, 4);
    assert_eq!(image.height, 2);
    assert_eq!(image.bits, 16);
    assert_eq!(image.samples, le(&[0, 1, 2, 3, 32768, 40000, 65534, 65535]));
    assert_eq!(
        image.cards,
        vec![
            ("XBINNING".to_string(), CardValue::Int(1)),
            ("YBINNING".to_string(), CardValue::Int(1)),
            ("CCD-TEMP".to_string(), CardValue::Real(-9.5)),
            ("BAYERPAT".to_string(), CardValue::Text("RGGB".to_string())),
        ]
    );
}

/// US3-S1: `decode` refuses a file the console did not write, each with its
/// text: zero bytes, a 3-axis file, `BITPIX` -32, `BZERO` 0, and a frame cut
/// off 8 bytes after its header.
#[test]
fn decode_refuses_what_the_console_did_not_write() {
    assert_eq!(refusal(&[0; 2880]), "not a FITS image");
    assert_eq!(
        refusal(&frame_of(&worked_cards(&[("NAXIS", "3")]))),
        "a 3-axis FITS file is not one image"
    );
    assert_eq!(
        refusal(&frame_of(&worked_cards(&[("BITPIX", "-32")]))),
        "unsupported FITS BITPIX -32"
    );
    assert_eq!(
        refusal(&frame_of(&worked_cards(&[("BZERO", "0")]))),
        "unsupported FITS BZERO 0"
    );
    assert_eq!(
        refusal(&frame_of(&worked_cards(&[]))[..2888]),
        "the FITS data is short: 8 of 16 bytes"
    );
}
