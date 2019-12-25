// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::{
    BitWrap,
    BitWrapError,
};


/// Maximum bitrate descriptor.
///
/// ISO 13818-1 - 2.6.26
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc0E {
    #[bits(8, skip = 0x0E)]
    #[bits(8, skip = 3)]

    /// The value indicates an upper bound of the bitrate,
    /// including transport overhead, that will be encountered
    /// in this program element or program.
    #[bits(2, skip = 0b11)]
    #[bits(22)]
    pub bitrate: u32
}


impl std::convert::TryFrom<&[u8]> for Desc0E {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc0E {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + 3 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 2 + 3, 0x00);
        self.pack(&mut buffer[skip ..]).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
        crate::psi::Desc0E,
    };

    static DATA: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];

    #[test]
    fn test_0e_parse() {
        let desc = Desc0E::try_from(DATA).unwrap();

        assert_eq!(desc.bitrate, 77500);
    }

    #[test]
    fn test_0e_assemble() {
        let desc = Desc0E {
            bitrate: 77500
        };

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
