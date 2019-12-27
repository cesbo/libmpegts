// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc83i {
    #[bits(16)]
    pub service_id: u16,

    #[bits(1)]
    pub visible: u8,

    #[bits(5, skip = 0b11111)]
    #[bits(10)]
    pub lcn: u16,
}


/// Logical Channel Descriptor - provides a default channel number label for service
///
/// HD-BOOK-DTT - 7.3.1
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc83 {
    #[bits(8, skip = 0x83)]
    #[bits(8, name = desc_len, value = self.items.len() * 4)]

    /// List of pairs service_id (pnr), visible flag, and channel number
    #[bytes(desc_len)]
    pub items: Vec<Desc83i>,
}


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::psi::{
            Desc83,
            Desc83i,
        },
    };

    static DATA: &[u8] = &[
        0x83, 0x08, 0x21, 0x85, 0xfc, 0x19, 0x21, 0x86,
        0xfc, 0x2b,
    ];

    #[test]
    fn test_83_parse() {
        let mut desc = Desc83::default();
        desc.unpack(DATA).unwrap();

        let mut items = desc.items.iter();

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8581);
        assert_eq!(item.visible, 1);
        assert_eq!(item.lcn, 25);

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8582);
        assert_eq!(item.visible, 1);
        assert_eq!(item.lcn, 43);
    }

    #[test]
    fn test_83_assemble() {
        let desc = Desc83 {
            items: vec![
                Desc83i {
                    service_id: 8581,
                    visible: 1,
                    lcn: 25,
                },
                Desc83i {
                    service_id: 8582,
                    visible: 1,
                    lcn: 43,
                },
            ]
        };

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
