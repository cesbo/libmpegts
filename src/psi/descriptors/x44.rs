// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::{
    bytes::Bytes,
    psi::BCD,
};

use super::Desc;


const MIN_SIZE: usize = 13;


/// Cable delivery system descriptor.
///
/// EN 300 468 - 6.2.13.1
#[derive(Debug, Default)]
pub struct Desc44 {
    /// Frequency in Hz.
    pub frequency: u32,
    /// Outer FEC scheme.
    pub fec_outer: u8,
    /// Modulation scheme used on a cable delivery system.
    pub modulation: u8,
    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    pub symbol_rate: u32,
    /// Inner FEC scheme.
    pub fec: u8
}


impl Desc44 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: slice[2 ..].get_u32().from_bcd() * 100,
            fec_outer: slice[7] & 0x0F,
            modulation: slice[8],
            symbol_rate: slice[9 ..].get_u24().from_bcd(),
            fec: slice[12] & 0x0F
        }
    }
}


impl Desc for Desc44 {
    #[inline]
    fn tag(&self) -> u8 {
        0x44
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x44;
        buffer[skip + 1] = (size - 2) as u8;
        buffer[skip + 2 ..].set_u32((self.frequency / 100).to_bcd());
        buffer[skip + 6] = 0xFF;  // reserved
        buffer[skip + 7] = 0xF0 | self.fec_outer;  // reserved + fec outer
        buffer[skip + 8] = self.modulation;
        buffer[skip + 9 ..].set_u24(self.symbol_rate.to_bcd());
        buffer[skip + 12] = self.fec;
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        constants,
        psi::{
            Descriptors,
            Desc44,
        },
    };

    static DATA_44: &[u8] = &[
        0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0, 0x05, 0x00, 0x68, 0x75, 0x00];

    #[test]
    fn test_44_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_44);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc44>();
        assert_eq!(desc.frequency, 346000000);
        assert_eq!(desc.fec_outer, constants::FEC_OUTER_NOT_DEFINED);
        assert_eq!(desc.modulation, constants::MODULATION_DVB_C_256_QAM);
        assert_eq!(desc.symbol_rate, 6875);
        assert_eq!(desc.fec, constants::FEC_NOT_DEFINED);
    }

    #[test]
    fn test_44_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc44 {
            frequency: 346000000,
            fec_outer: constants::FEC_OUTER_NOT_DEFINED,
            modulation: constants::MODULATION_DVB_C_256_QAM,
            symbol_rate: 6875,
            fec: constants::FEC_NOT_DEFINED
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_44);
    }
}
