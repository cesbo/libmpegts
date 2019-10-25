// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 2;


#[derive(Debug, Clone)]
pub struct Desc0Ai {
    pub code: StringDVB,
    pub audio_type: u8,
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


impl Desc0A {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE &&
        ((slice.len() - 2) % 4) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;

        while slice.len() > skip {
            let code = StringDVB::from(&slice[skip .. skip + 3]);
            let audio_type = slice[skip + 3];
            result.items.push(Desc0Ai {
                code,
                audio_type,
            });
            skip += 4;
        }
        result
    }
}


impl Desc for Desc0A {
    #[inline]
    fn tag(&self) -> u8 {
        0x0A
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.items.len() * 4
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0A);
        buffer.push((self.size() - 2) as u8);

        for item in &self.items {
            item.code.assemble(buffer);
            buffer.push(item.audio_type);
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        textcode,
        psi::{
            Descriptors,
            Desc0A,
            Desc0Ai,
        },
    };

    static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];

    #[test]
    fn test_0a_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_0A);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc0A>();
        let item = &desc.items[0];
        assert_eq!(item.code, textcode::StringDVB::from_str("eng", 0));
        assert_eq!(item.audio_type, 1);
    }

    #[test]
    fn test_0a_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc0A {
            items: vec![
                Desc0Ai {
                    code: textcode::StringDVB::from_str("eng", 0),
                    audio_type: 1
                },
            ]
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_0A);
    }
}
