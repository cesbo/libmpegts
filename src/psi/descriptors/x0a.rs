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


#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc0Ai {
    // TODO: replace with builtin [u8; N]
    #[bits(24, from = Self::from_language_code, into = Self::into_language_code)]
    pub code: [u8; 3],

    #[bits(8)]
    pub audio_type: u8,
}


impl Desc0Ai {
    #[inline]
    fn from_language_code(value: u32) -> [u8; 3] {
        [
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        ]
    }

    #[inline]
    fn into_language_code(value: [u8; 3]) -> u32 {
        (u32::from(value[0]) << 16) |
        (u32::from(value[1]) << 8) |
        (u32::from(value[2]))
    }
}


/// The language descriptor is used to specify the language
/// of the associated program element.
///
/// ISO 13818-1 - 2.6.18
#[derive(Debug, Default, Clone)]
pub struct Desc0A {
    /// 0 - Identifies the language or languages used by the associated program element
    /// 1 - Type of audio stream
    pub items: Vec<Desc0Ai>
}


impl BitWrap for Desc0A {
    fn pack(&self, dst: &mut [u8]) -> Result<usize, BitWrapError> {
        let mut skip = 2;

        if dst.len() < 2 {
            return Err(BitWrapError);
        }

        for item in &self.items {
            skip += item.pack(&mut dst[skip ..])?;
        }

        dst[0] = 0x0A;
        dst[1] = (skip - 2) as u8;

        Ok(skip)
    }

    fn unpack(&mut self, src: &[u8]) -> Result<usize, BitWrapError> {
        let mut skip = 2;

        while src.len() > skip {
            let mut item = Desc0Ai::default();
            skip += item.unpack(&src[skip ..])?;
            self.items.push(item);
        }

        Ok(skip)
    }
}


impl std::convert::TryFrom<&[u8]> for Desc0A {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc0A {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.items.len() * 4 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);
        self.pack(&mut buffer[skip ..]).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
        crate::{
            psi::{
                Desc0A,
                Desc0Ai,
            },
        },
    };

    static DATA: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];

    #[test]
    fn test_0a_parse() {
        let desc = Desc0A::try_from(DATA).unwrap();

        let item = &desc.items[0];
        assert_eq!(&item.code, b"eng");
        assert_eq!(item.audio_type, 1);
    }

    #[test]
    fn test_0a_assemble() {
        let desc = Desc0A {
            items: vec![
                Desc0Ai {
                    code: *b"eng",
                    audio_type: 1
                },
            ]
        };

        let mut assembled: Vec<u8> = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
