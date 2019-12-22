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
pub struct Desc41i {
    #[bits(16)]
    pub service_id: u16,
    #[bits(8)]
    pub service_type: u8,
}


impl std::convert::TryFrom<&[u8]> for Desc41i {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


/// Service List Descriptor - provides a means of listing the services by
/// service_id and service type
///
/// EN 300 468 - 6.2.35
#[derive(Debug, Default, Clone)]
pub struct Desc41 {
    /// List of pairs service_id (pnr) and service_type
    pub items: Vec<Desc41i>,
}


impl std::convert::TryFrom<&[u8]> for Desc41 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        let mut skip = 2;
        while value.len() > skip {
            result.items.push(Desc41i::try_from(&value[skip ..])?);
            skip += 3;
        }

        Ok(result)
    }
}


impl Desc41 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.items.len() * 3 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x41;
        buffer[skip + 1] = (size - 2) as u8;
        skip += 2;

        for item in &self.items {
            item.pack(&mut buffer[skip ..]).unwrap();
            skip += 3;
        }
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
        crate::psi::{
            Desc41,
            Desc41i,
        },
    };

    static DATA: &[u8] = &[0x41, 0x06, 0x21, 0x85, 0x01, 0x21, 0x86, 0x01];

    #[test]
    fn test_41_parse() {
        let desc = Desc41::try_from(DATA).unwrap();

        let mut items = desc.items.iter();

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8581);
        assert_eq!(item.service_type, 1);

        let item = items.next().unwrap();
        assert_eq!(item.service_id, 8582);
        assert_eq!(item.service_type, 1);
    }

    #[test]
    fn test_41_assemble() {
        let desc = Desc41 {
            items: vec![
                Desc41i {
                    service_id: 8581,
                    service_type: 1,
                },
                Desc41i {
                    service_id: 8582,
                    service_type: 1,
                },
            ]
        };

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
