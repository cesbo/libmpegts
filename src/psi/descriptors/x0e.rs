// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::bytes::*;
use super::Desc;


const MIN_SIZE: usize = 5;


/// Maximum bitrate descriptor.
///
/// ISO 13818-1 - 2.6.26
#[derive(Debug, Default, Clone)]
pub struct Desc0E {
    /// The value indicates an upper bound of the bitrate,
    /// including transport overhead, that will be encountered
    /// in this program element or program.
    pub bitrate: u32
}


impl Desc0E {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            bitrate: slice[2 ..].get_u24() & 0x003F_FFFF,
        }
    }
}


impl Desc for Desc0E {
    #[inline]
    fn tag(&self) -> u8 {
        0x0E
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x0E;
        buffer[skip + 1] = (size - 2) as u8;
        buffer[skip + 2 ..].set_u24(0x00C0_0000 | self.bitrate);
    }
}


#[cfg(test)]
mod tests {
    use crate::psi::{
        Descriptors,
        Desc0E,
    };

    static DATA_0E: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];

    #[test]
    fn test_0e_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_0E);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc0E>();
        assert_eq!(desc.bitrate, 77500);
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
