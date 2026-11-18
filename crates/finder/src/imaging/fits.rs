//! FITS images as the console writes them (031): one image, a dozen cards and
//! big-endian samples, written by hand because one image needs no library
//! (owner decision 5). US3 adds reading back.

/// A card's value as the console writes it.
#[derive(Debug, Clone, PartialEq)]
pub enum CardValue {
    Logical(bool),
    Int(i64),
    Real(f64),
    Text(String),
}

/// One FITS image: `bits` 8 or 16, little-endian samples.
#[derive(Debug, Clone, PartialEq)]
pub struct FitsImage {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    pub samples: Vec<u8>,
    /// The cards after the structural ones, in order.
    pub cards: Vec<(String, CardValue)>,
}

/// One 80-byte card: the keyword padded to 8, `= ` in columns 9 and 10, then
/// a logical or a number right-justified to column 30, or a text in single
/// quotes from column 11, padded inside the quotes to 8 characters, quotes
/// doubled, cut at 68 (031 US3-S1 pins the columns).
fn card(keyword: &str, value: &CardValue) -> [u8; 80] {
    let mut card = [b' '; 80];
    let keyword = keyword.as_bytes();
    let width = keyword.len().min(8);
    card[..width].copy_from_slice(&keyword[..width]);
    card[8] = b'=';
    card[9] = b' ';
    let text = match value {
        CardValue::Logical(true) => "T".to_string(),
        CardValue::Logical(false) => "F".to_string(),
        CardValue::Int(n) => n.to_string(),
        // A real is written as Rust prints it, with the exponent upper-case.
        CardValue::Real(n) => format!("{n:?}").replace('e', "E"),
        CardValue::Text(s) => {
            let mut inner = s.replace('\'', "''");
            if inner.len() < 8 {
                inner.push_str(&" ".repeat(8 - inner.len()));
            }
            if inner.len() > 66 {
                inner.truncate(66);
            }
            format!("'{inner}'")
        }
    };
    let bytes = text.as_bytes();
    let start = if matches!(value, CardValue::Text(_)) || bytes.len() >= 20 {
        10
    } else {
        30 - bytes.len()
    };
    card[start..start + bytes.len()].copy_from_slice(bytes);
    card
}

/// One FITS image as bytes: the structural cards, then `cards`, then `END`, the
/// header padded with spaces and the data with zeros to multiples of 2880. A
/// 16-bit sample `v` is stored as the big-endian `i16` `v − 32768`; an 8-bit
/// sample as it is.
pub fn encode(image: &FitsImage) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&card("SIMPLE", &CardValue::Logical(true)));
    header.extend_from_slice(&card("BITPIX", &CardValue::Int(i64::from(image.bits))));
    header.extend_from_slice(&card("NAXIS", &CardValue::Int(2)));
    header.extend_from_slice(&card("NAXIS1", &CardValue::Int(i64::from(image.width))));
    header.extend_from_slice(&card("NAXIS2", &CardValue::Int(i64::from(image.height))));
    if image.bits == 16 {
        header.extend_from_slice(&card("BZERO", &CardValue::Int(32768)));
        header.extend_from_slice(&card("BSCALE", &CardValue::Int(1)));
    }
    for (keyword, value) in &image.cards {
        header.extend_from_slice(&card(keyword, value));
    }
    let mut end = [b' '; 80];
    end[..3].copy_from_slice(b"END");
    header.extend_from_slice(&end);
    while header.len() % 2880 != 0 {
        header.push(b' ');
    }
    let mut data = Vec::new();
    match image.bits {
        16 => {
            for sample in image.samples.as_chunks::<2>().0 {
                let value = u16::from_le_bytes([sample[0], sample[1]]);
                let stored = (i32::from(value) - 32768) as i16;
                data.extend_from_slice(&stored.to_be_bytes());
            }
        }
        _ => data.extend_from_slice(&image.samples),
    }
    while data.len() % 2880 != 0 {
        data.push(0);
    }
    header.extend(data);
    header
}

/// Why bytes are not an image `encode` writes. Displays as its text.
#[derive(Debug, Clone, PartialEq)]
pub struct FitsError(pub String);

impl std::fmt::Display for FitsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FitsError {}

/// A card's keyword: its first 8 bytes, trimmed.
fn keyword(card: &[u8]) -> String {
    String::from_utf8_lossy(&card[..8]).trim().to_string()
}

/// A card's value as `decode` reads it back: a text in single quotes, its
/// doubled quotes made single and its trailing spaces dropped; `T` or `F`; a
/// number with no `.`, `E` or `e` as an `Int`; any other number as a `Real`.
/// `None` when the value is none of these.
fn value(text: &str) -> Option<CardValue> {
    let bytes = text.as_bytes();
    if bytes.first() == Some(&b'\'') {
        let mut inner = Vec::new();
        let mut index = 1;
        while index < bytes.len() {
            if bytes[index] == b'\'' {
                if bytes.get(index + 1) == Some(&b'\'') {
                    inner.push(b'\'');
                    index += 2;
                } else {
                    return Some(CardValue::Text(
                        String::from_utf8_lossy(&inner).trim_end().to_string(),
                    ));
                }
            } else {
                inner.push(bytes[index]);
                index += 1;
            }
        }
        None
    } else if text == "T" {
        Some(CardValue::Logical(true))
    } else if text == "F" {
        Some(CardValue::Logical(false))
    } else if text.contains('.') || text.contains('E') || text.contains('e') {
        text.parse::<f64>().ok().map(CardValue::Real)
    } else {
        text.parse::<i64>().ok().map(CardValue::Int)
    }
}

/// One card of a header as `decode` reads it: its keyword and, for a card
/// with `= ` in columns 9 and 10, the trimmed text of its value.
struct RawCard {
    keyword: String,
    value: Option<String>,
}

/// The header's cards up to `END`, and where the data begins: at the end of the
/// 2880-byte block that holds the `END` card. `None` when no `END` is found.
struct Header {
    cards: Vec<RawCard>,
    data_start: usize,
}

/// The header of `bytes`, `None` when no `END` card is found.
fn header(bytes: &[u8]) -> Option<Header> {
    let mut cards = Vec::new();
    for (block, block_bytes) in bytes.chunks(2880).enumerate() {
        for card in block_bytes.chunks(80) {
            if card.len() < 80 {
                break;
            }
            let keyword = keyword(card);
            if keyword == "END" {
                return Some(Header {
                    cards,
                    data_start: (block + 1) * 2880,
                });
            }
            let value = if &card[8..10] == b"= " {
                Some(String::from_utf8_lossy(&card[10..]).trim().to_string())
            } else {
                None
            };
            cards.push(RawCard { keyword, value });
        }
    }
    None
}

/// A structural card's whole number, `None` when the card is absent or holds
/// no whole number.
fn whole(cards: &[RawCard], name: &str) -> Option<i64> {
    let text = cards
        .iter()
        .find(|card| card.keyword == name)?
        .value
        .as_deref()?;
    match value(text) {
        Some(CardValue::Int(n)) => Some(n),
        _ => None,
    }
}

/// The raw text a structural card's value was written with, `0` when the card
/// is absent.
fn written<'a>(cards: &'a [RawCard], name: &str) -> &'a str {
    cards
        .iter()
        .find(|card| card.keyword == name)
        .and_then(|card| card.value.as_deref())
        .unwrap_or("0")
}

/// Read back an image `encode` writes, 8-bit or 16-bit, with the cards after
/// the structural ones in their order, and refuse anything else with a
/// `FitsError` that displays the texts of US3-S1 (FR-016).
pub fn decode(bytes: &[u8]) -> Result<FitsImage, FitsError> {
    let header = header(bytes).ok_or(FitsError("not a FITS image".to_string()))?;
    let simple = header.cards.first().is_some_and(|card| {
        card.keyword == "SIMPLE"
            && card
                .value
                .as_deref()
                .and_then(value)
                .is_some_and(|value| value == CardValue::Logical(true))
    });
    if !simple {
        return Err(FitsError("not a FITS image".to_string()));
    }
    let naxis = whole(&header.cards, "NAXIS").unwrap_or(0);
    if naxis != 2 {
        return Err(FitsError(format!(
            "a {naxis}-axis FITS file is not one image"
        )));
    }
    let bits = whole(&header.cards, "BITPIX").unwrap_or(0);
    if bits != 8 && bits != 16 {
        return Err(FitsError(format!("unsupported FITS BITPIX {bits}")));
    }
    if bits == 16 && whole(&header.cards, "BZERO") != Some(32768) {
        return Err(FitsError(format!(
            "unsupported FITS BZERO {}",
            written(&header.cards, "BZERO")
        )));
    }
    let dimension = |name: &str| {
        whole(&header.cards, name)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
    };
    let (width, height) = (dimension("NAXIS1"), dimension("NAXIS2"));
    let need = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(bits as u64 / 8);
    let have = bytes.len().saturating_sub(header.data_start) as u64;
    if have < need {
        return Err(FitsError(format!(
            "the FITS data is short: {have} of {need} bytes"
        )));
    }
    let data = &bytes[header.data_start.min(bytes.len())..];
    let samples = if bits == 8 {
        data[..need as usize].to_vec()
    } else {
        data[..need as usize]
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|chunk| {
                let stored = i16::from_be_bytes([chunk[0], chunk[1]]);
                ((i32::from(stored) + 32768) as u16).to_le_bytes()
            })
            .collect()
    };
    let structural = [
        "SIMPLE", "BITPIX", "NAXIS", "NAXIS1", "NAXIS2", "BZERO", "BSCALE",
    ];
    let cards = header
        .cards
        .into_iter()
        .filter(|card| !structural.contains(&card.keyword.as_str()))
        .filter_map(|card| {
            let value = card.value.as_deref().and_then(value)?;
            Some((card.keyword, value))
        })
        .collect();
    Ok(FitsImage {
        width,
        height,
        bits: bits as u32,
        samples,
        cards,
    })
}

/// `YYYY-MM-DD` and `YYYY-MM-DDTHH:MM:SS.mmm` of console Unix seconds, in UTC
/// to the millisecond (FR-012's `DATE-OBS`).
pub fn utc(unix_s: f64) -> (String, String) {
    let millis = (unix_s * 1000.0).round() as i64;
    let (days, secs, ms) = (
        millis.div_euclid(86_400_000),
        millis.rem_euclid(86_400_000) / 1000,
        millis.rem_euclid(1000),
    );
    // Days since 1970-01-01 to a civil date (Howard Hinnant's civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = format!("{year:04}-{month:02}-{day:02}");
    let (hour, minute, second) = (secs / 3600, secs / 60 % 60, secs % 60);
    let datetime = format!("{date}T{hour:02}:{minute:02}:{second:02}.{ms:03}");
    (date, datetime)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-16T02:00:00Z, the sim rig's start (the shared setup).
    #[test]
    fn utc_formats_the_rig_start_to_the_millisecond() {
        assert_eq!(utc(1_768_528_800.0).0, "2026-01-16");
        assert_eq!(utc(1_768_528_800.0).1, "2026-01-16T02:00:00.000");
        assert_eq!(utc(1_768_528_800.5).1, "2026-01-16T02:00:00.500");
        assert_eq!(utc(1_768_528_810.0).1, "2026-01-16T02:00:10.000");
    }
}
