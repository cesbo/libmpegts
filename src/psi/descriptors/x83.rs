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


impl std::convert::TryFrom<&[u8]> for Desc83i {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


/// Logical Channel Descriptor - provides a default channel number label for service
///
/// HD-BOOK-DTT - 7.3.1
#[derive(Debug, Default, Clone)]
pub struct Desc83 {
    /// List of pairs service_id (pnr), visible flag, and channel number
    pub items: Vec<Desc83i>,
}


impl BitWrap for Desc83 {
    fn pack(&self, dst: &mut [u8]) -> Result<usize, BitWrapError> {
        let mut skip = 2;

        if dst.len() < 2 {
            return Err(BitWrapError);
        }

        for item in &self.items {
            skip += item.pack(&mut dst[skip ..])?;
        }

        dst[0] = 0x83;
        dst[1] = (skip - 2) as u8;

        Ok(skip)
    }

    fn unpack(&mut self, src: &[u8]) -> Result<usize, BitWrapError> {
        let mut skip = 2;

        while src.len() > skip {
            let mut item = Desc83i::default();
            skip += item.unpack(&src[skip ..])?;
            self.items.push(item);
        }

        Ok(skip)
    }
}


impl std::convert::TryFrom<&[u8]> for Desc83 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc83 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.items.len() * 4 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);
        self.pack(&mut buffer[skip ..]).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
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
        let desc = Desc83::try_from(DATA).unwrap();

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

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
