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
