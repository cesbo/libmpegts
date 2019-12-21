// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrapError;


#[derive(Debug, Clone)]
pub struct Desc0Ai {
    pub code: [u8; 3],
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


impl std::convert::TryFrom<&[u8]> for Desc0A {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if (value.len() - 2) % 4 != 0 {
            return Err(BitWrapError);
        }

        let mut result = Self::default();
        let mut skip = 2;

        while value.len() > skip {
            result.items.push(Desc0Ai {
                code: [
                    value[skip    ],
                    value[skip + 1],
                    value[skip + 2],
                ],
                audio_type: value[skip + 3],
            });

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
    use bitwrap::BitWrap;

    use crate::{
        psi::{
            Descriptor,
            Descriptors,
            Desc0A,
            Desc0Ai,
        },
    };

    static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];

    #[test]
    fn test_0a_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.unpack(DATA_0A).unwrap();

        let mut iter = descriptors.iter();
        if let Some(Descriptor::Desc0A(desc)) = iter.next() {
            let item = &desc.items[0];
            assert_eq!(&item.code, b"eng");
            assert_eq!(item.audio_type, 1);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_0a_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc0A {
            items: vec![
                Desc0Ai {
                    code: *b"eng",
                    audio_type: 1
                },
            ]
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_0A);
    }
}
