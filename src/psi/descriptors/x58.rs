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

use crate::{
    psi::{
        BCDTime,
        MJDFrom,
        MJDTo,
    },
};


#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc58i {
    #[bits_convert(24, Self::from_country_code, Self::into_country_code)]
    pub country_code: [u8; 3],

    #[bits(6)]
    pub region_id: u8,

    #[bits_skip(1, 0b1)]

    #[bits(1)]
    pub offset_polarity: u8,

    #[bits_convert(16, u16::from_bcd_time, u16::to_bcd_time)]
    pub offset: u16,

    #[bits_convert(40, Self::from_time, Self::into_time)]
    pub time_of_change: u64,

    #[bits_convert(16, u16::from_bcd_time, u16::to_bcd_time)]
    pub next_offset: u16,
}


impl Desc58i {
    #[inline]
    fn from_country_code(value: u32) -> [u8; 3] {
        [
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        ]
    }

    #[inline]
    fn into_country_code(value: [u8; 3]) -> u32 {
        (u32::from(value[0]) << 16) |
        (u32::from(value[1]) << 8) |
        (u32::from(value[2]))
    }

    #[inline]
    fn from_time(value: u64) -> u64 {
        ((value >> 24) as u16).from_mjd() +
        u64::from(((value & 0xFFFFFF) as u32).from_bcd_time())
    }

    #[inline]
    fn into_time(value: u64) -> u64 {
        (u64::from(value.to_mjd()) << 24) |
        u64::from((value as u32).to_bcd_time())
    }
}


impl std::convert::TryFrom<&[u8]> for Desc58i {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


/// The local time offset descriptor may be used in the TOT to describe country specific
/// dynamic changes of the local time offset relative to UTC.
///
/// EN 300 468 - 6.2.20
#[derive(Debug, Default, Clone)]
pub struct Desc58 {
    pub items: Vec<Desc58i>
}


impl std::convert::TryFrom<&[u8]> for Desc58 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        let mut skip = 2;
        while value.len() > skip {
            result.items.push(Desc58i::try_from(&value[skip ..])?);
            skip += 13;
        }

        Ok(result)
    }
}


impl Desc58 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.items.len() * 13 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x58;
        buffer[skip + 1] = (size - 2) as u8;
        skip += 2;

        for item in &self.items {
            item.pack(&mut buffer[skip ..]).unwrap();
            skip += 13;
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
            Desc58,
            Desc58i,
        },
    };

    static DATA_58: &[u8] = &[
        0x58, 0x1a,
        0x47, 0x42, 0x52, 0x02, 0x00, 0x00, 0xda, 0xcb, 0x00, 0x59, 0x59, 0x01, 0x00,
        0x49, 0x52, 0x4c, 0x02, 0x00, 0x00, 0xda, 0xcb, 0x00, 0x59, 0x59, 0x01, 0x00];

    #[test]
    fn test_58_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.unpack(DATA_58).unwrap();

        let mut iter = descriptors.iter();
        if let Some(Descriptor::Desc58(desc)) = iter.next() {
            assert_eq!(desc.items.len(), 2);

            let item = desc.items.get(0).unwrap();
            assert_eq!(&item.country_code, b"GBR");
            assert_eq!(item.region_id, 0);
            assert_eq!(item.offset_polarity, 0);
            assert_eq!(item.offset, 0);
            assert_eq!(item.time_of_change, 1332637199);
            assert_eq!(item.next_offset, 60);

            let item = desc.items.get(1).unwrap();
            assert_eq!(&item.country_code, b"IRL");
            assert_eq!(item.region_id, 0);
            assert_eq!(item.offset_polarity, 0);
            assert_eq!(item.offset, 0);
            assert_eq!(item.time_of_change, 1332637199);
            assert_eq!(item.next_offset, 60);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_58_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc58 {
            items: vec! [
                Desc58i {
                    country_code: *b"GBR",
                    region_id: 0,
                    offset_polarity: 0,
                    offset: 0,
                    time_of_change: 1332637199,
                    next_offset: 60,
                },
                Desc58i {
                    country_code: *b"IRL",
                    region_id: 0,
                    offset_polarity: 0,
                    offset: 0,
                    time_of_change: 1332637199,
                    next_offset: 60,
                },
            ],
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_58);
    }
}
