//! Reading a FITS BLOB (plan.md § Reading a FITS BLOB): one image, a dozen
//! cards and big-endian samples, no library. The camera's own words are the
//! only thing read here; the samples change byte order and `BZERO` and
//! nothing else (spec FR-028).

/// One FITS image as a camera sends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// Little-endian samples, row by row from the top-left.
    pub samples: Vec<u8>,
    /// Keyword and value text, quotes and spaces trimmed, in file order.
    pub cards: Vec<(String, String)>,
}

impl Picture {
    /// The value text of the first card of this keyword, if the header has
    /// one: the empty text for a card without a value (`COMMENT`,
    /// `HISTORY`, a blank keyword), which no reader takes for a value.
    pub fn card(&self, keyword: &str) -> Option<&str> {
        self.cards
            .iter()
            .find(|(name, _)| name == keyword)
            .map(|(_, value)| value.as_str())
    }
}

/// The value text of one card, quotes and spaces trimmed: `None` when its
/// columns 9 and 10 are not `= ` (`COMMENT`, `HISTORY`, a blank keyword) or
/// when the value is empty. A value ends at a ` /` outside quotes; a quoted
/// value ends at its closing quote, `''` reading as one quote (plan.md §
/// Reading a FITS BLOB).
fn value_of(card: &[u8]) -> Option<String> {
    if card.len() < 10 || &card[8..10] != b"= " {
        return None;
    }
    let rest = &card[10..];
    let bytes: Vec<u8> = if rest.first() == Some(&b'\'') {
        // The closing quote: `''` inside is one quote, not the end.
        let mut cursor = 1;
        let mut inner: Vec<u8> = Vec::new();
        while cursor < rest.len() {
            match rest[cursor] {
                b'\'' => {
                    if rest.get(cursor + 1) == Some(&b'\'') {
                        inner.push(b'\'');
                        cursor += 2;
                    } else {
                        break;
                    }
                }
                byte => {
                    inner.push(byte);
                    cursor += 1;
                }
            }
        }
        inner
    } else {
        // An unquoted value ends at a ` /`, the FITS comment: a slash a
        // space precedes, at the card's end when there is none.
        let cut = rest
            .iter()
            .position(|&byte| byte == b'/')
            .filter(|&at| at > 0 && rest[at - 1] == b' ')
            .unwrap_or(rest.len());
        rest[..cut].to_vec()
    };
    let text = std::str::from_utf8(&bytes).unwrap_or_default();
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

impl Picture {
    fn number(&self, keyword: &str) -> Option<f64> {
        self.card(keyword)
            .and_then(|text| text.trim().parse::<f64>().ok())
    }
}

/// Read one FITS image as a camera sends it: 80-byte cards to `END`, the
/// header padded to a multiple of 2880 bytes, then `NAXIS1 × NAXIS2` samples
/// of `BITPIX` bits, big-endian and `BZERO`-shifted. The texts of the
/// refusals are plan.md § Reading a FITS BLOB's.
pub fn read(bytes: &[u8]) -> Result<Picture, String> {
    const NOT_VALID: &str = "the camera's FITS header is not valid";
    // The header: whole cards to `END`.
    let mut cards: Vec<(String, String)> = Vec::new();
    let mut cursor = 0usize;
    let end = loop {
        let Some(card) = bytes.get(cursor..cursor + 80) else {
            return Err(NOT_VALID.to_string());
        };
        let keyword = String::from_utf8_lossy(&card[..8]).trim().to_string();
        if keyword == "END" {
            break cursor + 80;
        }
        let value = value_of(card);
        cards.push((keyword, value.unwrap_or_default()));
        cursor += 80;
    };
    let header = Picture {
        width: 0,
        height: 0,
        bits: 0,
        samples: Vec::new(),
        cards,
    };
    if header.card("SIMPLE") != Some("T") {
        return Err(NOT_VALID.to_string());
    }
    let axes = header
        .number("NAXIS")
        .ok_or_else(|| NOT_VALID.to_string())?;
    if axes != 2.0 {
        return Err(format!("the camera sent a {}-axis FITS picture", axes));
    }
    let bits = header
        .number("BITPIX")
        .ok_or_else(|| NOT_VALID.to_string())?;
    if bits != 8.0 && bits != 16.0 {
        return Err(format!("unsupported FITS BITPIX {bits}"));
    }
    let width = header
        .number("NAXIS1")
        .ok_or_else(|| NOT_VALID.to_string())?;
    let height = header
        .number("NAXIS2")
        .ok_or_else(|| NOT_VALID.to_string())?;
    // The header is padded to a multiple of 2880 bytes, and the data follows.
    let data = end.div_ceil(2880) * 2880;
    let width = width as u32;
    let height = height as u32;
    let sample_bytes = bits.abs() as usize / 8;
    let needed = usize::try_from(width)
        .ok()
        .and_then(|width| usize::try_from(height).ok().map(|h| width * h))
        .and_then(|samples| samples.checked_mul(sample_bytes))
        .ok_or_else(|| NOT_VALID.to_string())?;
    let Some(data) = bytes.get(data..data + needed) else {
        return Err("the camera's FITS data is short".to_string());
    };
    let samples = match bits as u32 {
        8 => data.to_vec(),
        _ => {
            // For each big-endian signed sample, `clamp(s + BZERO, 0, 65535)`
            // as little-endian, `BZERO` being 0 when absent.
            let bzero = header.number("BZERO").unwrap_or(0.0);
            let mut out = Vec::with_capacity(needed);
            for pair in data.as_chunks::<2>().0 {
                let sample = i16::from_be_bytes(*pair) as f64;
                let value = (sample + bzero).clamp(0.0, 65535.0) as u16;
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        }
    };
    Ok(Picture {
        width,
        height,
        bits: bits as u32,
        samples,
        cards: header.cards,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One 80-byte card: the keyword in eight columns, `= ` in nine and ten,
    /// a number right-justified to column 30, a text quoted from column 11.
    fn card(keyword: &str, value: &str) -> [u8; 80] {
        let mut out = [b' '; 80];
        let head = format!("{keyword:<8}= ");
        out[..head.len()].copy_from_slice(head.as_bytes());
        let tail = match value
            .strip_prefix('\'')
            .and_then(|text| text.strip_suffix('\''))
        {
            Some(text) => format!("'{text:<8}'"),
            None => format!("{value:>width$}", value = value, width = 80 - head.len()),
        };
        out[head.len()..head.len() + tail.len()].copy_from_slice(tail.as_bytes());
        out
    }

    /// One 80-byte card with no `= ` in columns 9 and 10, as `COMMENT` and
    /// `HISTORY` are written: whatever it holds, it has no value, an
    /// apostrophe included.
    fn bare(keyword: &str, text: &str) -> [u8; 80] {
        let mut out = [b' '; 80];
        let head = format!("{keyword:<8} {text}");
        out[..head.len()].copy_from_slice(head.as_bytes());
        out
    }

    /// A whole FITS image: the cards, `END`, the header padded to 2880
    /// bytes, then the data padded to the same.
    fn fits(cards: &[[u8; 80]], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for card in cards {
            out.extend_from_slice(card);
        }
        out.extend_from_slice(&end());
        while out.len() % 2880 != 0 {
            out.push(b' ');
        }
        out.extend_from_slice(data);
        while out.len() % 2880 != 0 {
            out.push(0);
        }
        out
    }

    /// The `END` card: the keyword alone in its eight columns.
    fn end() -> [u8; 80] {
        let mut out = [b' '; 80];
        out[..3].copy_from_slice(b"END");
        out
    }

    /// The header's cards are read in order, values trimmed, comments cut,
    /// `COMMENT` and `HISTORY` without a value, `''` inside a quote one
    /// quote, and numbers in E notation (plan.md § Reading a FITS BLOB; the
    /// saved capture's own cards).
    #[test]
    fn the_header_is_read_as_the_capture_writes_it() {
        let bytes = fits(
            &[
                card("SIMPLE", "T"),
                bare("COMMENT", "an apostrophe's comment"),
                card("BITPIX", "16"),
                card("NAXIS", "2"),
                card("NAXIS1", "64"),
                card("NAXIS2", "48"),
                card("EXTEND", "T"),
                card("BAYERPAT", "'RGGB'"),
                card("FRAME", "'Light Frame'"),
                card("DATE-OBS", "'2026-11-17T03:58:48.785'"),
                card("CCD-TEMP", "0.000E+00"),
                card("GAIN", "9.000E+01"),
                bare("COMMENT", "Generated by INDI"),
            ],
            &[0u8; 64 * 48 * 2],
        );
        let picture = read(&bytes).unwrap();
        assert_eq!(picture.card("SIMPLE"), Some("T"));
        assert_eq!(picture.card("COMMENT"), Some(""));
        assert_eq!(picture.card("BITPIX"), Some("16"));
        assert_eq!(picture.card("BAYERPAT"), Some("RGGB"));
        assert_eq!(picture.card("FRAME"), Some("Light Frame"));
        assert_eq!(picture.card("DATE-OBS"), Some("2026-11-17T03:58:48.785"));
        assert_eq!(picture.card("CCD-TEMP"), Some("0.000E+00"));
        assert_eq!(picture.width, 64);
        assert_eq!(picture.height, 48);
        assert_eq!(picture.bits, 16);
        assert_eq!(picture.samples.len(), 64 * 48 * 2);
        // The keyword order is the file's.
        assert_eq!(picture.cards[0].0, "SIMPLE");
        assert_eq!(picture.cards[1].0, "COMMENT");
        assert_eq!(picture.cards[2].0, "BITPIX");
    }

    /// `BITPIX` 16 samples come off the byte order and `BZERO` and touch
    /// nothing else; `BITPIX` 8 gives the bytes as they are.
    #[test]
    fn samples_change_byte_order_and_bzero_and_nothing_else() {
        // The worked frame of plan.md § The worked FITS frame: the stored
        // samples 8000 8001 8002 8003 0000 1C40 7FFE 7FFF.
        let stored = [
            0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE,
            0x7F, 0xFF,
        ];
        let bytes = fits(
            &[
                card("SIMPLE", "T"),
                card("BITPIX", "16"),
                card("NAXIS", "2"),
                card("NAXIS1", "4"),
                card("NAXIS2", "2"),
                card("BZERO", "32768"),
            ],
            &stored,
        );
        let picture = read(&bytes).unwrap();
        let samples: Vec<u16> = picture
            .samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        assert_eq!(samples, [0, 1, 2, 3, 32768, 40000, 65534, 65535]);

        let mono = fits(
            &[
                card("SIMPLE", "T"),
                card("BITPIX", "8"),
                card("NAXIS", "2"),
                card("NAXIS1", "3"),
                card("NAXIS2", "2"),
            ],
            &[0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF],
        );
        let picture = read(&mono).unwrap();
        assert_eq!(picture.bits, 8);
        assert_eq!(picture.samples, [0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF]);
    }

    /// Each refusal answers its own text: no `END`, `SIMPLE` not `T`, an
    /// axis count other than two, a `BITPIX` other than 8 and 16, and data
    /// shorter than the header says (plan.md § Reading a FITS BLOB).
    #[test]
    fn each_broken_picture_answers_its_own_text() {
        let cards = |naxis: &str, bitpix: &str| {
            vec![
                card("SIMPLE", "T"),
                card("BITPIX", bitpix),
                card("NAXIS", naxis),
                card("NAXIS1", "4"),
                card("NAXIS2", "2"),
            ]
        };
        let no_end = cards("2", "16").concat();
        assert_eq!(
            read(&no_end),
            Err("the camera's FITS header is not valid".to_string())
        );
        let mut not_simple = fits(&cards("2", "16"), &[0; 16]);
        not_simple[29] = b'F';
        assert_eq!(
            read(&not_simple),
            Err("the camera's FITS header is not valid".to_string())
        );
        assert_eq!(
            read(&fits(&cards("3", "16"), &[0; 24])),
            Err("the camera sent a 3-axis FITS picture".to_string())
        );
        assert_eq!(
            read(&fits(&cards("2", "-32"), &[0; 32])),
            Err("unsupported FITS BITPIX -32".to_string())
        );
        // Fewer data bytes than NAXIS1 × NAXIS2 × |BITPIX|/8: a header that
        // promises 64 × 48 with no data behind it.
        let short = fits(
            &[
                card("SIMPLE", "T"),
                card("BITPIX", "16"),
                card("NAXIS", "2"),
                card("NAXIS1", "64"),
                card("NAXIS2", "48"),
            ],
            &[],
        );
        assert_eq!(
            read(&short),
            Err("the camera's FITS data is short".to_string())
        );
        assert_eq!(
            read(&short),
            Err("the camera's FITS data is short".to_string())
        );
    }
}
