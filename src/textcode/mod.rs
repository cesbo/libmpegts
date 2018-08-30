mod data;
use textcode::data::*;

use std::char;

/// Coding of text characters
/// Possible codepage values:
///
/// * `0` - `ISO6937` Latin superset of ISO/IEC 6937 with addition of the Euro symbol
/// * `1` - `ISO8859-1` Western European
/// * `2` - `ISO8859-2` Central European
/// * `3` - `ISO8859-3` South European
/// * `4` - `ISO8859-4` North European
/// * `5` - `ISO8859-5` Cyrillic
/// * `6` - `ISO8859-6` Arabic
/// * `7` - `ISO8859-7` Greek
/// * `8` - `ISO8859-8` Hebrew
/// * `9` - `ISO8859-9` Turkish
/// * `10` - `ISO8859-10` Nordic
/// * `11` - `ISO8859-11` Thai
/// * `13` - `ISO8859-13` Baltic Rim
/// * `14` - `ISO8859-14` Celtic
/// * `15` - `ISO8859-15` Western European
/// * `16` - `ISO8859-16` South-Eastern European
/// * `21` - `UTF-8`

#[inline]
fn get_codepage_map(codepage: usize) -> Option<&'static [u16]> {
    match codepage {
        0 => Some(&ISO6937),
        1 => Some(&ISO8859_1),
        2 => Some(&ISO8859_2),
        3 => Some(&ISO8859_3),
        4 => Some(&ISO8859_4),
        5 => Some(&ISO8859_5),
        6 => Some(&ISO8859_6),
        7 => Some(&ISO8859_7),
        8 => Some(&ISO8859_8),
        9 => Some(&ISO8859_9),
        10 => Some(&ISO8859_10),
        11 => Some(&ISO8859_11),
        13 => Some(&ISO8859_13),
        14 => Some(&ISO8859_14),
        15 => Some(&ISO8859_15),
        16 => Some(&ISO8859_16),
        _ => None,
    }
}

fn decode_codepage(dst: &mut String, data: &[u8], codepage: usize) {
    if data.len() == 0 {
        return;
    }

    let map = match get_codepage_map(codepage) {
        Some(v) => v,
        None => return,
    };

    for &c in data.iter() {
        if c <= 0x7F {
            dst.push(c as char);
        } else if c >= 0xA0 {
            match map[c as usize - 0xA0] {
                0 => dst.push('?'),
                u => dst.push(unsafe { char::from_u32_unchecked(u as u32) }),
            };
        }
    }
}

/// Decode DVB string into UTF-8 string
/// Supports next codepages: ISO6937, ISO8859, UTF-8
/// Detect codepage automaticaly, based on codepage identifier
/// Push decoded string into `text` option and returns codepage identifier
///
/// # Examples
///
/// Decode iso6937:
///
/// ```
/// use mpegts::textcode;
/// let data = b"Hello!";
/// let mut text = String::new();
/// let codepage = textcode::decode(&mut text, data);
/// assert_eq!(codepage, 0);
/// assert_eq!(text.as_str(), "Hello!");
/// ```
///
/// Decode iso8859-5:
///
/// ```
/// use mpegts::textcode;
/// let iso8859_5: Vec<u8> = vec![0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
/// let mut text = String::new();
/// let codepage = textcode::decode(&mut text, &iso8859_5);
/// assert_eq!(codepage, 5);
/// assert_eq!(text.as_str(), "Привет!");
/// ```
pub fn decode(text: &mut String, data: &[u8]) -> usize {
    if data.len() == 0 {
        0
    } else if data[0] == 21 {
        /* UTF-8 */
        text.push_str(&String::from_utf8_lossy(&data[1 ..]));
        21
    } else if data[0] >= 0x20 {
        /* ISO6937 */
        decode_codepage(text, data, 0);
        0
    } else if data[0] < 0x10 {
        let codepage = data[0] as usize + 4;
        decode_codepage(text, &data[1 ..], codepage);
        codepage
    } else if data[0] == 0x10 && data.len() >= 3 {
        let codepage = data[2] as usize;
        decode_codepage(text, &data[3 ..], codepage);
        codepage
    } else {
        text.push('?');
        0
    }
}

/// Encode UTF-8 string into DVB string
/// Prepend codepage identifier if codepage not 0 (ISO6937)
///
/// # Examples
///
/// Encode iso6937:
///
/// ```
/// use mpegts::textcode;
/// let text = "Hello!";
/// let mut out: Vec<u8> = Vec::new();
/// textcode::encode(&text, &mut out, 0);
/// assert_eq!(out, text.as_bytes());
/// ```
///
/// Encode iso8859-5:
///
/// ```
/// use mpegts::textcode;
/// let text = "Привет!";
/// let mut out: Vec<u8> = Vec::new();
/// textcode::encode(&text, &mut out, 5);
/// let iso8859_5: Vec<u8> = vec![0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
/// assert_eq!(out, iso8859_5);
/// ```
///
/// Encode utf-8:
///
/// ```
/// use mpegts::textcode;
/// let text = "Привет!";
/// let mut out: Vec<u8> = Vec::new();
/// textcode::encode(&text, &mut out, 21);
/// assert_eq!(out[0], 21);
/// assert_eq!(&out[1 ..], text.as_bytes());
/// ```
pub fn encode(text: &str, data: &mut Vec<u8>, codepage: usize) {
    if text.len() == 0 {
        return;
    }

    // UTF-8
    if codepage == 21 {
        data.push(21);
        data.extend_from_slice(text.as_bytes());
        return;
    }

    let map = match get_codepage_map(codepage) {
        Some(v) => v,
        None => return,
    };

    if codepage > 0 {
        data.push(0x10);
        data.push(0x00);
        data.push(codepage as u8);
    }

    for c in text.chars() {
        if c <= 0x7F as char {
            data.push(c as u8);
        } else if c >= 0xA0 as char {
            match map.iter().position(|&u| u == c as u16) {
                Some(v) => data.push((v as u8) + 0xA0),
                None => data.push('?' as u8),
            };
        }
    }
}
