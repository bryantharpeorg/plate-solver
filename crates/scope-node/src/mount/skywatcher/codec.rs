//! Wire format from Sky-Watcher's motor-controller command set §§4–5.
use super::Error;

const HEX: &[u8; 16] = b"0123456789ABCDEF";
pub fn encode_u24(value: u32) -> [u8; 6] {
    std::array::from_fn(|i| {
        let shift = (i / 2) * 8 + if i % 2 == 0 { 4 } else { 0 };
        HEX[((value >> shift) & 15) as usize]
    })
}
pub fn hex(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::InvalidReply),
    }
}
pub fn decode_u24(data: &[u8]) -> Result<u32, Error> {
    if data.len() != 6 {
        return Err(Error::InvalidReply);
    }
    let mut value = 0;
    for (i, pair) in data.as_chunks::<2>().0.iter().enumerate() {
        value |= u32::from(hex(pair[0])? * 16 + hex(pair[1])?) << (8 * i);
    }
    Ok(value)
}
pub fn encode_position(position: i32) -> [u8; 6] {
    encode_u24(position.wrapping_add(0x80_0000) as u32)
}
pub fn decode_position(data: &[u8]) -> Result<i32, Error> {
    Ok(decode_u24(data)? as i32 - 0x80_0000)
}
pub fn frame(command: u8, axis: u8, data: &[u8]) -> Result<Vec<u8>, Error> {
    if !command.is_ascii_alphabetic()
        || !(1..=3).contains(&axis)
        || data.len() > 6
        || data.iter().any(|b| hex(*b).is_err())
    {
        return Err(Error::InvalidReply);
    }
    let mut bytes = vec![b':', command, b'0' + axis];
    bytes.extend_from_slice(data);
    bytes.push(b'\r');
    Ok(bytes)
}
pub fn reply(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.last() != Some(&b'\r') || bytes.len() < 2 {
        return Err(Error::InvalidReply);
    }
    let data = &bytes[1..bytes.len() - 1];
    if data.len() > 6 || data.iter().any(|b| hex(*b).is_err()) {
        return Err(Error::InvalidReply);
    }
    match bytes[0] {
        b'=' => Ok(data.to_vec()),
        b'!' if data.len() == 1 => Err(Error::Refused(hex(data[0])?)),
        _ => Err(Error::InvalidReply),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sky-Watcher command set §4–5: little-endian byte pairs, position bias.
    #[test]
    fn positions_match_published_byte_order_and_offset() {
        for (position, wire) in [(0, b"000080"), (1, b"010080"), (-1, b"FFFF7F")] {
            assert_eq!(&encode_position(position), wire);
            assert_eq!(decode_position(wire).unwrap(), position);
        }
        assert_eq!(encode_u24(0x123456), *b"563412");
        assert_eq!(frame(b'S', 1, b"230180").unwrap(), b":S1230180\r");
    }

    #[test]
    fn every_24_bit_value_and_signed_position_round_trips() {
        for value in 0..=0xff_ffff {
            assert_eq!(decode_u24(&encode_u24(value)).unwrap(), value);
            let position = value as i32 - 0x80_0000;
            assert_eq!(
                decode_position(&encode_position(position)).unwrap(),
                position
            );
        }
    }

    #[test]
    fn framing_and_replies_reject_malformed_input() {
        assert_eq!(reply(b"=123ABC\r").unwrap(), b"123ABC");
        assert_eq!(reply(b"=\r").unwrap(), b"");
        assert!(matches!(
            reply(b"!2\r"),
            Err(super::super::Error::Refused(2))
        ));
        for bytes in [b"=X\r".as_slice(), b"=00", b"?\r", b"!\r", b"!222\r"] {
            assert!(reply(bytes).is_err(), "{bytes:?}");
        }
        for bytes in [b"GG0000".as_slice(), b"00000", b"00000000", &[255; 6]] {
            assert!(decode_u24(bytes).is_err());
        }
        assert!(frame(b'j', 0, b"").is_err());
        assert!(frame(b'j', 4, b"").is_err());
        assert!(frame(b'\r', 1, b"").is_err());
        assert!(frame(b'G', 1, b"0\r").is_err());
    }
}
