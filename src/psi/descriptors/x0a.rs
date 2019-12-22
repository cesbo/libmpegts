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


#[derive(Debug, Default, Clone)]
pub struct Desc0Ai {
    pub code: [u8; 3],
    pub audio_type: u8,
}


impl BitWrap for Desc0Ai {
    fn pack(&self, dst: &mut [u8]) -> Result<usize, BitWrapError> {
        if dst.len() < 4 {
            return Err(BitWrapError);
        }

        dst[0] = self.code[0];
        dst[1] = self.code[1];
        dst[2] = self.code[2];
        dst[3] = self.audio_type;

        Ok(4)
    }

    fn unpack(&mut self, src: &[u8]) -> Result<usize, BitWrapError> {
        if src.len() < 4 {
            return Err(BitWrapError);
        }

        self.code[0] = src[0];
        self.code[1] = src[1];
        self.code[2] = src[2];
        self.audio_type = src[3];

        Ok(4)
    }
}


impl std::convert::TryFrom<&[u8]> for Desc0Ai {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
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


impl std::convert::TryFrom<&[u8]> for Desc0A {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        let mut skip = 2;

        while value.len() > skip {
            result.items.push(Desc0Ai::try_from(&value[skip ..])?);
            skip += 4;
        }

        Ok(result)
    }
}


impl Desc0A {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.items.len() * 4 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0A);
        buffer.push((self.size() - 2) as u8);

        for item in &self.items {
            buffer.extend_from_slice(&item.code);
            buffer.push(item.audio_type);
        }
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
