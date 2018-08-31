mod data;

use std::char;

/// Latin superset of ISO/IEC 6937 with addition of the Euro symbol
pub const ISO6937: usize = 0;

/// Western European
pub const ISO8859_1: usize = 1;

/// Central European
pub const ISO8859_2: usize = 2;

/// South European
pub const ISO8859_3: usize = 3;

/// North European
pub const ISO8859_4: usize = 4;

/// Cyrillic
pub const ISO8859_5: usize = 5;

/// Arabic
pub const ISO8859_6: usize = 6;

/// Greek
pub const ISO8859_7: usize = 7;

/// Hebrew
pub const ISO8859_8: usize = 8;

/// Turkish
pub const ISO8859_9: usize = 9;

/// Nordic
pub const ISO8859_10: usize = 10;

/// Thai
pub const ISO8859_11: usize = 11;

/// Baltic Rim
pub const ISO8859_13: usize = 13;

/// Celtic
pub const ISO8859_14: usize = 14;

/// Western European
pub const ISO8859_15: usize = 15;

/// South-Eastern European
pub const ISO8859_16: usize = 16;

/// UTF-8
pub const UTF8: usize = 21;

#[inline]
fn get_codepage_map(codepage: usize) -> Option<&'static [u16]> {
    match codepage {
        ISO6937 => Some(&data::ISO6937),
        ISO8859_1 => Some(&data::ISO8859_1),
        ISO8859_2 => Some(&data::ISO8859_2),
        ISO8859_3 => Some(&data::ISO8859_3),
        ISO8859_4 => Some(&data::ISO8859_4),
        ISO8859_5 => Some(&data::ISO8859_5),
        ISO8859_6 => Some(&data::ISO8859_6),
        ISO8859_7 => Some(&data::ISO8859_7),
        ISO8859_8 => Some(&data::ISO8859_8),
        ISO8859_9 => Some(&data::ISO8859_9),
        ISO8859_10 => Some(&data::ISO8859_10),
        ISO8859_11 => Some(&data::ISO8859_11),
        ISO8859_13 => Some(&data::ISO8859_13),
        ISO8859_14 => Some(&data::ISO8859_14),
        ISO8859_15 => Some(&data::ISO8859_15),
        ISO8859_16 => Some(&data::ISO8859_16),
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
/// assert_eq!(codepage, textcode::ISO6937);
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
/// assert_eq!(codepage, textcode::ISO8859_5);
/// assert_eq!(text.as_str(), "Привет!");
/// ```
pub fn decode(text: &mut String, data: &[u8]) -> usize {
    if data.len() == 0 {
        0
    } else if data[0] == UTF8 as u8 {
        /* UTF-8 */
        text.push_str(&String::from_utf8_lossy(&data[1 ..]));
        UTF8
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
/// textcode::encode(&text, &mut out, textcode::ISO6937);
/// assert_eq!(out, text.as_bytes());
/// ```
///
/// Encode iso8859-5:
///
/// ```
/// use mpegts::textcode;
/// let text = "Привет!";
/// let mut out: Vec<u8> = Vec::new();
/// textcode::encode(&text, &mut out, textcode::ISO8859_5);
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
/// textcode::encode(&text, &mut out, textcode::UTF8);
/// assert_eq!(out[0] as usize, textcode::UTF8);
/// assert_eq!(&out[1 ..], text.as_bytes());
/// ```
pub fn encode(text: &str, data: &mut Vec<u8>, codepage: usize) {
    if text.len() == 0 {
        return;
    }

    if codepage == UTF8 {
        data.push(UTF8 as u8);
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
