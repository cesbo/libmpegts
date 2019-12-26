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
    #[bits(8, skip = 0x0E)]
    #[bits(8, skip = 3)]

    /// The value indicates an upper bound of the bitrate,
    /// including transport overhead, that will be encountered
    /// in this program element or program.
    #[bits(2, skip = 0b11)]
    #[bits(22)]
    pub bitrate: u32
}


impl Desc0E {
    pub fn new(bitrate: u32) -> Self {
        Self {
            bitrate,
        }
    }
}


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::psi::Desc0E,
    };

    static DATA: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];

    #[test]
    fn test_0e_unpack() {
        let mut desc = Desc0E::default();
        desc.unpack(DATA).unwrap();

        assert_eq!(desc.bitrate, 77500);
    }

    #[test]
    fn test_0e_pack() {
        let desc = Desc0E::new(77500);

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
