// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::{
    textcode::StringDVB,
    bytes::Bytes,
    psi::{
        BCDTime,
        MJDFrom,
        MJDTo,
    },
};

use super::Desc;


const MIN_SIZE: usize = 2;


#[derive(Debug, Default)]
pub struct Desc58i {
    pub country_code: StringDVB,
    pub region_id: u8,
    pub offset_polarity: u8,
    pub offset: u16,
    pub time_of_change: u64,
    pub next_offset: u16,
}


/// The local time offset descriptor may be used in the TOT to describe country specific
/// dynamic changes of the local time offset relative to UTC.
///
/// EN 300 468 - 6.2.20
#[derive(Debug, Default)]
pub struct Desc58 {
    pub items: Vec<Desc58i>
}


impl Desc58 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE &&
        ((slice.len() - 2) % 13) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;

        while slice.len() > skip {
            let country_code = StringDVB::from(&slice[skip .. skip + 3]);
            let region_id = slice[skip + 3] >> 2;
            let offset_polarity = slice[skip + 3] & 0x01;
            let offset = slice[skip + 4 ..].get_u16().from_bcd_time();
            let time_of_change =
                slice[skip + 6 ..].get_u16().from_mjd() +
                u64::from(slice[skip + 8 ..].get_u24().from_bcd_time());
            let next_offset = slice[skip + 11 ..].get_u16().from_bcd_time();

            result.items.push(Desc58i {
                country_code,
                region_id,
                offset_polarity,
                offset,
                time_of_change,
                next_offset,
            });

            skip += 13;
        }

        result
    }
}


impl Desc for Desc58 {
    #[inline]
    fn tag(&self) -> u8 { 0x58 }

    #[inline]
    fn size(&self) -> usize { MIN_SIZE + self.items.len() * 13 }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x58);
        buffer.push((self.size() - 2) as u8);

        for item in &self.items {
            item.country_code.assemble(buffer);

            let skip = buffer.len();
            buffer.resize(skip + 10, 0x00);

            buffer[skip] = item.region_id << 2 | 0x02 | item.offset_polarity;
            buffer[skip + 1 ..].set_u16((item.offset).to_bcd_time());
            buffer[skip + 3 ..].set_u16(item.time_of_change.to_mjd());
            buffer[skip + 5 ..].set_u24((item.time_of_change as u32).to_bcd_time());
            buffer[skip + 8 ..].set_u16((item.next_offset).to_bcd_time());
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        textcode,
        psi::{
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
        descriptors.parse(DATA_58);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc58>();
        assert_eq!(desc.items.len(), 2);

        let item = desc.items.get(0).unwrap();
        assert_eq!(item.country_code, textcode::StringDVB::from_str("GBR", textcode::ISO6937));
        assert_eq!(item.region_id, 0);
        assert_eq!(item.offset_polarity, 0);
        assert_eq!(item.offset, 0);
        assert_eq!(item.time_of_change, 1332637199);
        assert_eq!(item.next_offset, 60);

        let item = desc.items.get(1).unwrap();
        assert_eq!(item.country_code, textcode::StringDVB::from_str("IRL", textcode::ISO6937));
        assert_eq!(item.region_id, 0);
        assert_eq!(item.offset_polarity, 0);
        assert_eq!(item.offset, 0);
        assert_eq!(item.time_of_change, 1332637199);
        assert_eq!(item.next_offset, 60);
    }

    #[test]
    fn test_58_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc58 {
            items: vec! [
                Desc58i {
                    country_code: textcode::StringDVB::from_str("GBR", textcode::ISO6937),
                    region_id: 0,
                    offset_polarity: 0,
                    offset: 0,
                    time_of_change: 1332637199,
                    next_offset: 60,
                },
                Desc58i {
                    country_code: textcode::StringDVB::from_str("IRL", textcode::ISO6937),
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
