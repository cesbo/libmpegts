// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


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
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc0A {
    #[bits(8, skip = 0x0A)]

    #[bits(8, value = self.items.len() * 4)]
    len: u8,

    #[bytes(self.len)]
    pub items: Vec<Desc0Ai>
}


impl Desc0A {
    pub fn new(items: Vec<Desc0Ai>) -> Self {
        Self {
            len: 0,
            items,
        }
    }
}


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::{
            psi::{
                Desc0A,
                Desc0Ai,
            },
        },
    };

    static DATA: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];

    #[test]
    fn test_0a_unpack() {
        let mut desc = Desc0A::default();
        desc.unpack(DATA).unwrap();

        let item = &desc.items[0];
        assert_eq!(&item.code, b"eng");
        assert_eq!(item.audio_type, 1);
    }

    #[test]
    fn test_0a_pack() {
        let desc = Desc0A::new(vec![
            Desc0Ai {
                code: *b"eng",
                audio_type: 1
            },
        ]);

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
