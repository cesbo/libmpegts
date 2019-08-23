// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::bytes::*;
use super::Desc;


const MIN_SIZE: usize = 2;


#[derive(Debug)]
pub struct Desc83i {
    pub service_id: u16,
    pub visible: u8,
    pub lcn: u16,
}


/// Logical Channel Descriptor - provides a default channel number label for service
///
/// HD-BOOK-DTT - 7.3.1
#[derive(Debug, Default)]
pub struct Desc83 {
    /// List of pairs service_id (pnr), visible flag, and channel number
    pub items: Vec<Desc83i>,
}


impl Desc83 {
    #[inline]
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE &&
        ((slice.len() - 2) % 4) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;
        while slice.len() >= skip + 4 {
            let service_id = slice[skip ..].get_u16();
            let visible = slice[skip + 2] >> 7;
            let lcn = slice[skip + 2 ..].get_u16() & 0x03FF;
            result.items.push(Desc83i {
                service_id,
                visible,
                lcn,
            });
            skip += 4;
        }
        result
    }
}


impl Desc for Desc83 {
    #[inline]
    fn tag(&self) -> u8 {
        0x83
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.items.len() * 4
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x83;
        buffer[skip + 1] = (size - 2) as u8;
        skip += 2;

        for item in &self.items {
            buffer[skip ..].set_u16(item.service_id);
            buffer[skip + 2 ..].set_u16(
                set_bits!(16,
                    u16::from(item.visible), 1,
                    0x1F, 5,
                    item.lcn, 10));
            skip += 4;
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::psi::{
        Descriptors,
        Desc83,
        Desc83i,
    };

    static DATA_83: &[u8] = &[0x83, 0x08, 0x21, 0x85, 0xfc, 0x19, 0x21, 0x86, 0xfc, 0x2b];

    #[test]
    fn test_83_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_83);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc83>();
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
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc83 {
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
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_83);
    }
}
