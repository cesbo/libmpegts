pub use textcode::dvb::Charset;

pub struct TextcodeRef<'a>(Charset, &'a [u8]);

impl<'a> TextcodeRef<'a> {
    pub fn charset(&self) -> Charset {
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
            return Ok(TextcodeRef(Charset::Utf8, &[]));
        }

        let charset = match data[0] {
            0x00 => Charset::Iso6937,
            0x20 ..= 0xFF => Charset::Iso6937,

            0x01 => Charset::Iso8859_5,
            0x02 => Charset::Iso8859_6,
            0x03 => Charset::Iso8859_7,
            0x04 => Charset::Iso8859_8,
            0x05 => Charset::Iso8859_9,
            0x06 => Charset::Iso8859_10,
            0x07 => Charset::Iso8859_11,
            0x09 => Charset::Iso8859_13,
            0x0A => Charset::Iso8859_14,
            0x0B => Charset::Iso8859_15,

            0x10 => {
                if data.len() < 3 {
                    return Err(TextcodeError::InvalidLength);
                }
                let part = u16::from_be_bytes([data[1], data[2]]);
                match part {
                    0x01 => Charset::Iso8859_1,
                    0x02 => Charset::Iso8859_2,
                    0x03 => Charset::Iso8859_3,
                    0x04 => Charset::Iso8859_4,
                    0x05 => Charset::Iso8859_5,
                    0x06 => Charset::Iso8859_6,
                    0x07 => Charset::Iso8859_7,
                    0x08 => Charset::Iso8859_8,
                    0x09 => Charset::Iso8859_9,
                    0x0A => Charset::Iso8859_10,
                    0x0B => Charset::Iso8859_11,
                    0x0D => Charset::Iso8859_13,
                    0x0E => Charset::Iso8859_14,
                    0x0F => Charset::Iso8859_15,
                    0x10 => Charset::Iso8859_16,

                    _ => return Err(TextcodeError::InvalidCodepage),
                }
            }

            0x11 => Charset::Utf16,
            0x13 => Charset::Gb2312,
            0x15 => Charset::Utf8,
            0x1E => Charset::Geo,

            _ => return Err(TextcodeError::InvalidCodepage),
        };

        Ok(TextcodeRef(charset, data))
    }
}
