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


/// Satellite delivery system descriptor.
///
/// EN 300 468 - 6.2.13.2
#[derive(Debug, Default)]
pub struct Desc43 {
    /// Frequency in KHz.
    pub frequency: u32,
    /// Position in minutes of angle.
    pub orbital_position: u16,
    /// Satellite position in the western or eastern part of the orbit.
    pub west_east_flag: u8,
    /// Polarization of the transmitted signal.
    pub polarization: u8,
    /// Roll-off factor used in DVB-S2.
    pub rof: u8,
    /// Broadcast scheme used on a satellite delivery system.
    /// DVB-S2 or not.
    pub s2: u8,
    /// Modulation scheme used on a satellite delivery system.
    pub modulation: u8,
    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    pub symbol_rate: u32,
    /// Inner FEC scheme.
    pub fec: u8
}


impl Desc43 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: slice[2 ..].get_u32().from_bcd() * 10,
            orbital_position: slice[6 ..].get_u16().from_bcd() * 6,
            west_east_flag: (slice[8] & 0b1000_0000) >> 7,
            polarization: (slice[8] & 0b0110_0000) >> 5,
            rof: (slice[8] & 0b0001_1000) >> 3,
            s2: (slice[8] & 0b0000_0100) >> 2,
            modulation: slice[8] & 0b0000_0011,
            symbol_rate: slice[9 ..].get_u24().from_bcd(),
            fec: slice[12] & 0x0F
        }
    }
}


impl Desc for Desc43 {
    #[inline]
    fn tag(&self) -> u8 {
        0x43
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x43;
        buffer[skip + 1] = (size - 2) as u8;
        buffer[skip + 2 ..].set_u32((self.frequency / 10).to_bcd());
        buffer[skip + 6 ..].set_u16((self.orbital_position / 6).to_bcd());
        buffer[skip + 8] = set_bits!(8,
            self.west_east_flag, 1,
            self.polarization, 2,
            self.rof, 2,
            self.s2, 1,
            self.modulation, 2);
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
            Desc43,
        },
    };

    static DATA_43: &[u8] = &[0x43, 0x0b, 0x01, 0x23, 0x80, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03];

    #[test]
    fn test_43_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_43);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc43>();
        assert_eq!(desc.frequency, 12380000);
        assert_eq!(desc.orbital_position, 780);
        assert_eq!(desc.west_east_flag, constants::POSITION_EAST);
        assert_eq!(desc.polarization, constants::POLARIZATION_V);
        assert_eq!(desc.rof, 0);
        assert_eq!(desc.s2, 0);
        assert_eq!(desc.modulation, constants::MODULATION_DVB_S_QPSK);
        assert_eq!(desc.symbol_rate, 27500);
        assert_eq!(desc.fec, constants::FEC_3_4);
    }

    #[test]
    fn test_43_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc43 {
            frequency: 12380000,
            orbital_position: 780,
            west_east_flag: constants::POSITION_EAST,
            polarization: constants::POLARIZATION_V,
            rof: 0,
            s2: 0,
            modulation: constants::MODULATION_DVB_S_QPSK,
            symbol_rate: 27500,
            fec: constants::FEC_3_4
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_43);
    }
}
