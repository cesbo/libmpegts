// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

#![allow(dead_code)]

use std::{
    cmp,
    fmt
};
use textcode::*;

pub mod lang;


/// Latin superset of ISO/IEC 6937 with addition of the Euro symbol
pub const ISO6937: u8 = 0;

/// Western European
pub const ISO8859_1: u8 = 1;

/// Central European
pub const ISO8859_2: u8 = 2;

/// South European
pub const ISO8859_3: u8 = 3;

/// North European
pub const ISO8859_4: u8 = 4;

/// Cyrillic
pub const ISO8859_5: u8 = 5;

/// Arabic
pub const ISO8859_6: u8 = 6;

/// Greek
pub const ISO8859_7: u8 = 7;

/// Hebrew
pub const ISO8859_8: u8 = 8;

/// Turkish
pub const ISO8859_9: u8 = 9;

/// Nordic
pub const ISO8859_10: u8 = 10;

/// Thai
pub const ISO8859_11: u8 = 11;

/// Baltic Rim
pub const ISO8859_13: u8 = 13;

/// Celtic
pub const ISO8859_14: u8 = 14;

/// Western European
pub const ISO8859_15: u8 = 15;

/// UTF-8
pub const UTF8: u8 = 21;

fn encode_data(input_data: &str, codepage: u8) -> Vec<u8> {
    let mut dst: Vec<u8> = Vec::new();
    match codepage {
        ISO6937 => iso6937::encode(input_data, &mut dst),
        ISO8859_1 => iso8859_1::encode(input_data, &mut dst),
        ISO8859_2 => iso8859_2::encode(input_data, &mut dst),
        ISO8859_3 => iso8859_3::encode(input_data, &mut dst),
        ISO8859_4 => iso8859_4::encode(input_data, &mut dst),
        ISO8859_5 => iso8859_5::encode(input_data, &mut dst),
        ISO8859_6 => iso8859_6::encode(input_data, &mut dst),
        ISO8859_7 => iso8859_7::encode(input_data, &mut dst),
        ISO8859_8 => iso8859_8::encode(input_data, &mut dst),
        ISO8859_9 => iso8859_9::encode(input_data, &mut dst),
        ISO8859_10 => iso8859_10::encode(input_data, &mut dst),
        ISO8859_11 => iso8859_11::encode(input_data, &mut dst),
        ISO8859_13 => iso8859_13::encode(input_data, &mut dst),
        ISO8859_14 => iso8859_14::encode(input_data, &mut dst),
        ISO8859_15 => iso8859_15::encode(input_data, &mut dst),
        _ => dst.extend_from_slice(input_data.as_bytes()),
    }
    dst
}

fn decode_data(input_data: &[u8], codepage: u8) -> String {
    let mut dst = String::new();
    match codepage {
        ISO6937 => iso6937::decode(input_data, &mut dst),
        ISO8859_1 => iso8859_1::decode(input_data, &mut dst),
        ISO8859_2 => iso8859_2::decode(input_data, &mut dst),
        ISO8859_3 => iso8859_3::decode(input_data, &mut dst),
        ISO8859_4 => iso8859_4::decode(input_data, &mut dst),
        ISO8859_5 => iso8859_5::decode(input_data, &mut dst),
        ISO8859_6 => iso8859_6::decode(input_data, &mut dst),
        ISO8859_7 => iso8859_7::decode(input_data, &mut dst),
        ISO8859_8 => iso8859_8::decode(input_data, &mut dst),
        ISO8859_9 => iso8859_9::decode(input_data, &mut dst),
        ISO8859_10 => iso8859_10::decode(input_data, &mut dst),
        ISO8859_11 => iso8859_11::decode(input_data, &mut dst),
        ISO8859_13 => iso8859_13::decode(input_data, &mut dst),
        ISO8859_14 => iso8859_14::decode(input_data, &mut dst),
        ISO8859_15 => iso8859_15::decode(input_data, &mut dst),
        _ => dst = String::from_utf8_lossy(input_data).to_string(),
    };
    dst
}

#[derive(Default, Clone, PartialEq)]
pub struct StringDVB {
    codepage: u8,
    data: Vec<u8>,
}

impl fmt::Display for StringDVB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&decode_data(&self.data, self.codepage))
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
    pub fn from_str(s: &str, codepage: u8) -> Self {
        Self {
            codepage,
            data: encode_data(s, codepage)
        }
    }

    #[inline]
    pub fn get_codepage(&self) -> u8 {
        self.codepage
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns size in bytes that needed for assembled string
    /// Includes: codepage identifier, payload size
    #[inline]
    pub fn size(&self) -> usize {
        if self.data.is_empty() {
            0
        } else if self.codepage == ISO6937 {
            self.data.len()
        } else if self.codepage <= ISO8859_4 {
            3 + self.data.len()
        } else {
            1 + self.data.len()
        }
    }

    /// Writes text into buffer
    /// Prepends string size if `with_size` is `true`.
    /// Prepends codepage identifier if codepage is not ISO6937.
    /// Resulted buffer size should be greater or equal than `min` and less or
    /// equal than `max`.
    pub fn assemble(&self, dst: &mut Vec<u8>) {
        if self.data.is_empty() {
            return;
        }

        if self.codepage == ISO6937 {
            //
        } else if self.codepage <= ISO8859_4 {
            dst.push(0x10);
            dst.push(0x00);
            dst.push(self.codepage);
        } else if self.codepage <= ISO8859_15 {
            dst.push(self.codepage - 4);
        } else {
            dst.push(self.codepage);
        }

        dst.extend_from_slice(self.as_bytes());
    }

    pub fn assemble_sized(&self, dst: &mut Vec<u8>) {
        let size = dst.len();
        dst.push(0x00);
        self.assemble(dst);
        dst[size] = (dst.len() - size - 1) as u8;
    }

    pub fn truncate(&mut self, size: usize) {
        if self.data.len() > size {
            self.data.resize(size, 0);
            self.data[size - 1] = b'.';
            self.data[size - 2] = b'.';
            self.data[size - 3] = b'.';
        }
    }

    pub fn split(&self, size: usize) -> Vec<Self> {
        let size = match self.codepage {
            ISO6937 => size,
            UTF8 => size - 1,
            _ => size - 3,
        };

        let mut out: Vec<Self> = Vec::new();

        if self.is_empty() {
            out.push(self.clone());
            return out;
        }

        let mut skip = 0;
        while skip < self.data.len() {
            let mut next = cmp::min(self.data.len(), skip + size);
            if self.codepage == UTF8 && next < self.data.len() {
                for _ in 1 .. 3 {
                    if self.data[next] & 0xC0 != 0x80 {
                        break;
                    }
                    next -= 1;
                }
            }
            out.push(Self {
                codepage: self.codepage,
                data: self.data[skip .. next].to_vec(),
            });
            skip = next;
        }

        out
    }
}

impl<'a> From<&'a [u8]> for StringDVB {
    fn from(data: &[u8]) -> Self {
        if data.is_empty() {
            Self::default()
        } else if data[0] == UTF8 as u8 {
            Self {
                codepage: UTF8,
                data: Vec::from(&data[1 ..]),
            }
        } else if data[0] >= 0x20 {
            Self {
                codepage: ISO6937,
                data: Vec::from(data),
            }
        } else if data[0] < 0x10 {
            Self {
                codepage: data[0] + 4,
                data: Vec::from(&data[1 ..]),
            }
        } else if data[0] == 0x10 && data.len() >= 3 {
            Self {
                codepage: data[2],
                data: Vec::from(&data[3 ..]),
            }
        } else {
            Self {
                codepage: ISO6937,
                data: vec![b'?'],
            }
        }
    }
}

#[cfg(test)]
mod tests;
