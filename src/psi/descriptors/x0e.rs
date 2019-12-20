// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


/// Maximum bitrate descriptor.
///
/// ISO 13818-1 - 2.6.26
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc0E {
    #[bits_skip(8, 0x0E)]
    #[bits_skip(8, 3)]

    /// The value indicates an upper bound of the bitrate,
    /// including transport overhead, that will be encountered
    /// in this program element or program.
    #[bits_skip(2, 0b11)]
    #[bits(22)]
    pub bitrate: u32
}


impl Desc0E {
    pub (crate) fn parse(slice: &[u8]) -> Self {
        let mut x = Desc0E::default();
        x.unpack(slice).unwrap();
        x
    }

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
    use crate::psi::{
        Descriptor,
        Descriptors,
        Desc0E,
    };

    static DATA_0E: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];

    #[test]
    fn test_0e_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_0E);

        let mut iter = descriptors.iter();
        if let Some(Descriptor::Desc0E(desc)) = iter.next() {
            assert_eq!(desc.bitrate, 77500);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_0e_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc0E {
            bitrate: 77500
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_0E);
    }
}
