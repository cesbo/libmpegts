// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc41i {
    #[bits(16)]
    pub service_id: u16,

    #[bits(8)]
    pub service_type: u8,
}


/// Service List Descriptor - provides a means of listing the services by
/// service_id and service type
///
/// EN 300 468 - 6.2.35
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc41 {
    /// Type of CA system.
    #[bits(8, skip = 0x41)]

    #[bits(8, value = self.items.len() * 3)]
    len: usize,

    /// List of pairs service_id (pnr) and service_type
    #[bytes(self.len)]
    pub items: Vec<Desc41i>,
}


impl Desc41 {
    #[inline]
    pub fn new(items: Vec<Desc41i>) -> Self {
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
        crate::psi::{
            Desc41,
            Desc41i,
        },
    };

    static DATA: &[u8] = &[0x41, 0x06, 0x21, 0x85, 0x01, 0x21, 0x86, 0x01];

    #[test]
    fn test_41_unpack() {
        let mut desc = Desc41::default();
        desc.unpack(DATA).unwrap();

        let mut items = desc.items.iter();

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8581);
        assert_eq!(item.service_type, 1);

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8582);
        assert_eq!(item.service_type, 1);
    }

    #[test]
    fn test_41_pack() {
        let desc = Desc41::new(vec![
            Desc41i {
                service_id: 8581,
                service_type: 1,
            },
            Desc41i {
                service_id: 8582,
                service_type: 1,
            },
        ]);

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
