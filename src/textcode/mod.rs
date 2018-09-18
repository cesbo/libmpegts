#![allow(dead_code)]

mod data;
pub mod lang;

use std::char;
use std::fmt::{self, Write};

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

//

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

#[derive(Default)]
pub struct StringDVB {
    codepage: usize,
    data: Vec<u8>,
}

impl fmt::Display for StringDVB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.is_empty() {
            return Ok(());
        }

        if self.codepage == UTF8 {
            return f.write_str(&String::from_utf8_lossy(&self.data));
        }

        let map = match get_codepage_map(self.codepage) {
            Some(v) => v,
            None => return Err(fmt::Error),
        };

        for &c in &self.data {
            if c <= 0x7F {
                f.write_char(c as char)?;
            } else if c >= 0xA0 {
                match map[c as usize - 0xA0] {
                    0 => f.write_char('?'),
                    u => f.write_char(unsafe { char::from_u32_unchecked(u32::from(u)) }),
                }?;
            }
        }

        Ok(())
    }
}

impl fmt::Debug for StringDVB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StringDVB")
            .field("codepage", &self.codepage)
            .field("text", &self.to_string())
            .finish()
    }
}

impl StringDVB {
    /// Creates StringDVB from UTF-8 string
    pub fn from_str(s: &str, codepage: usize) -> Self {
        if codepage == UTF8 {
            return StringDVB {
                codepage,
                data: {
                    let mut data: Vec<u8> = Vec::new();
                    data.extend_from_slice(s.as_bytes());
                    data
                },
            }
        }

        let map = match get_codepage_map(codepage) {
            Some(v) => v,
            None => return StringDVB::from_str(s, UTF8),
        };

        StringDVB {
            codepage,
            data: {
                let mut data: Vec<u8> = Vec::new();
                for c in s.chars() {
                    if c <= 0x7F as char {
                        data.push(c as u8);
                    } else if c >= 0xA0 as char {
                        if let Some(v) = map.iter().position(|&u| u == c as u16) {
                            data.push((v as u8) + 0xA0);
                        } else {
                            data.push(b'?');
                        }
                    }
                }
                data
            }
        }
    }

    /// Creates StringDVB from DVB string
    pub fn from_raw(x: &[u8]) -> Self {
        if x.is_empty() {
            StringDVB::default()
        } else if x[0] == UTF8 as u8 {
            StringDVB {
                codepage: UTF8,
                data: Vec::from(&x[1 ..]),
            }
        } else if x[0] >= 0x20 {
            StringDVB {
                codepage: 0,
                data: Vec::from(x),
            }
        } else if x[0] < 0x10 {
            StringDVB {
                codepage: usize::from(x[0]) + 4,
                data: Vec::from(&x[1 ..]),
            }
        } else if x[0] == 0x10 && x.len() >= 3 {
            StringDVB {
                codepage: usize::from(x[2]),
                data: Vec::from(&x[3 ..]),
            }
        } else {
            StringDVB {
                codepage: 0,
                data: vec![b'?'],
            }
        }
    }

    #[inline]
    pub fn get_codepage(&self) -> usize {
        self.codepage
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn assemble(&self, dst: &mut Vec<u8>, with_size: bool) {
        let skip = dst.len();
        if with_size {
            dst.push(0x00);
        }

        if ! self.data.is_empty() {
            if self.codepage == UTF8 {
                dst.push(UTF8 as u8);
            } else if self.codepage != 0 {
                dst.push(0x10);
                dst.push(0x00);
                dst.push(self.codepage as u8);
            }

            dst.extend_from_slice(self.as_bytes());
        }

        if with_size {
            let size = dst.len() - skip - 1;
            dst[skip] = size as u8;
        }
    }
}
