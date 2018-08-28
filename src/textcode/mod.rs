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
#[derive(Default, Debug)]
pub struct StringDVB(usize, String);

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

impl ToString for StringDVB {
    #[inline]
    fn to_string(&self) -> String {
        self.1.to_string()
    }
}

impl StringDVB {
    #[inline]
    pub fn from_str(codepage: usize, string: &str) -> Self {
        StringDVB(codepage, string.to_string())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.1.as_str()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.1.len()
    }

    fn decode_codepage(&mut self, data: &[u8]) {
        if data.len() == 0 {
            return;
        }

        let map = match get_codepage_map(self.0) {
            Some(v) => v,
            None => return,
        };

        for c in data.iter() {
            let c = *c;

            if c <= 0x7F {
                self.1.push(c as char);
            } else if c >= 0xA0 {
                match map[c as usize - 0xA0] {
                    0 => self.1.push('?'),
                    u => self.1.push(unsafe { char::from_u32_unchecked(u as u32) }),
                };
            }
        }
    }

    /// Decode DVB string into UTF-8 string
    /// Supports next codepages: ISO6937, ISO8859, UTF-8
    /// Detect codepage automaticaly, based on codepage identifier
    ///
    /// # Examples
    ///
    /// Decode iso6937:
    ///
    /// ```
    /// use mpegts::textcode::*;
    /// let data = b"Hello!";
    /// let mut s = StringDVB::default();
    /// s.decode(data);
    /// assert_eq!(s.as_str(), "Hello!");
    /// ```
    ///
    /// Decode iso8859-5:
    ///
    /// ```
    /// use mpegts::textcode::*;
    /// let iso8859_5: Vec<u8> = vec![0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
    /// let mut s = StringDVB::default();
    /// s.decode(&iso8859_5);
    /// assert_eq!(s.as_str(), "Привет!");
    /// ```
    pub fn decode(&mut self, data: &[u8]) {
        if data.len() == 0 {
            return;
        } else if data[0] == 21 {
            /* UTF-8 */
            self.0 = 21;
            self.1.push_str(&String::from_utf8_lossy(&data[1 ..]));
        } else if data[0] >= 0x20 {
            /* ISO6937 */
            self.decode_codepage(data);
        } else if data[0] < 0x10 {
            self.0 = data[0] as usize + 4;
            self.decode_codepage(&data[1 ..]);
        } else if data[0] == 0x10 && data.len() >= 3 {
            self.0 = data[2] as usize;
            self.decode_codepage(&data[3 ..]);
        } else {
            self.1.push('?');
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
    /// use mpegts::textcode::*;
    /// let text = "Hello!";
    /// let string = StringDVB::from_str(0, text);
    /// let mut out: Vec<u8> = Vec::new();
    /// string.encode(&mut out);
    /// assert_eq!(out, text.as_bytes());
    /// ```
    ///
    /// Encode iso8859-5:
    ///
    /// ```
    /// use mpegts::textcode::*;
    /// let string = StringDVB::from_str(5, "Привет!");
    /// let mut out: Vec<u8> = Vec::new();
    /// string.encode(&mut out);
    /// let iso8859_5: Vec<u8> = vec![0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
    /// assert_eq!(out, iso8859_5);
    /// ```
    ///
    /// Encode utf-8:
    ///
    /// ```
    /// use mpegts::textcode::*;
    /// let string = StringDVB::from_str(21, "Привет!");
    /// let mut out: Vec<u8> = Vec::new();
    /// string.encode(&mut out);
    /// assert_eq!(out[0], 21);
    /// assert_eq!(&out[1 ..], string.as_str().as_bytes());
    /// ```
    pub fn encode(&self, dst: &mut Vec<u8>) {
        if self.1.len() == 0 {
            return;
        }

        // UTF-8
        if self.0 == 21 {
            dst.push(21);
            dst.extend_from_slice(self.1.as_bytes());
            return;
        }

        let map = match get_codepage_map(self.0) {
            Some(v) => v,
            None => return,
        };

        if self.0 > 0 {
            dst.push(0x10);
            dst.push(0x00);
            dst.push(self.0 as u8);
        }

        for c in self.1.chars() {
            if c <= 0x7F as char {
                dst.push(c as u8);
            } else if c >= 0xA0 as char {
                match map.iter().position(|&u| u == c as u16) {
                    Some(v) => dst.push((v as u8) + 0xA0),
                    None => dst.push('?' as u8),
                };
            }
        }
    }
}
