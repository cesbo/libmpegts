#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codepage {
    /// Latin superset of ISO/IEC 6937 with addition of the Euro symbol
    Iso6937,
    /// Western European
    Iso8859_1,
    /// Central European
    Iso8859_2,
    /// South European
    Iso8859_3,
    /// North European
    Iso8859_4,
    /// Cyrillic
    Iso8859_5,
    /// Arabic
    Iso8859_6,
    /// Greek
    Iso8859_7,
    /// Hebrew
    Iso8859_8,
    /// Turkish
    Iso8859_9,
    /// Nordic
    Iso8859_10,
    /// Thai
    Iso8859_11,
    /// Baltic Rim
    Iso8859_13,
    /// Celtic
    Iso8859_14,
    /// Western European
    Iso8859_15,
    /// South-Eastern European
    Iso8859_16,
    /// UTF-8
    Utf8,
    /// UTF-16
    Utf16,
    /// GB2312 (Simplified Chinese)
    Gb2312,
    /// Proprietary DVB single-byte Georgian
    Geo,
}

pub struct TextcodeRef<'a>(Codepage, &'a [u8]);

impl<'a> TextcodeRef<'a> {
    pub fn codepage(&self) -> Codepage {
        self.0
    }
}

impl<'a> std::fmt::Display for TextcodeRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        textcode::dvb::decode(self.1).fmt(f)
    }
}

#[derive(Debug)]
pub enum TextcodeError {
    InvalidLength,
    InvalidCodepage,
}

impl core::error::Error for TextcodeError {}

impl std::fmt::Display for TextcodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextcodeError::InvalidLength => write!(f, "Invalid length of input data"),
            TextcodeError::InvalidCodepage => write!(f, "Invalid or unsupported codepage"),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for TextcodeRef<'a> {
    type Error = TextcodeError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.is_empty() {
            return Ok(TextcodeRef(Codepage::Utf8, &[]));
        }

        let codepage = match data[0] {
            0x00 => Codepage::Iso6937,
            0x20 ..= 0xFF => Codepage::Iso6937,

            0x01 => Codepage::Iso8859_5,
            0x02 => Codepage::Iso8859_6,
            0x03 => Codepage::Iso8859_7,
            0x04 => Codepage::Iso8859_8,
            0x05 => Codepage::Iso8859_9,
            0x06 => Codepage::Iso8859_10,
            0x07 => Codepage::Iso8859_11,
            0x09 => Codepage::Iso8859_13,
            0x0A => Codepage::Iso8859_14,
            0x0B => Codepage::Iso8859_15,

            0x10 => {
                if data.len() < 3 {
                    return Err(TextcodeError::InvalidLength);
                }
                let part = u16::from_be_bytes([data[1], data[2]]);
                match part {
                    0x01 => Codepage::Iso8859_1,
                    0x02 => Codepage::Iso8859_2,
                    0x03 => Codepage::Iso8859_3,
                    0x04 => Codepage::Iso8859_4,
                    0x05 => Codepage::Iso8859_5,
                    0x06 => Codepage::Iso8859_6,
                    0x07 => Codepage::Iso8859_7,
                    0x08 => Codepage::Iso8859_8,
                    0x09 => Codepage::Iso8859_9,
                    0x0A => Codepage::Iso8859_10,
                    0x0B => Codepage::Iso8859_11,
                    0x0D => Codepage::Iso8859_13,
                    0x0E => Codepage::Iso8859_14,
                    0x0F => Codepage::Iso8859_15,
                    0x10 => Codepage::Iso8859_16,

                    _ => return Err(TextcodeError::InvalidCodepage),
                }
            }

            0x11 => Codepage::Utf16,
            0x13 => Codepage::Gb2312,
            0x15 => Codepage::Utf8,
            0x1E => Codepage::Geo,

            _ => return Err(TextcodeError::InvalidCodepage),
        };

        Ok(TextcodeRef(codepage, data))
    }
}
