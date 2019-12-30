// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::BCD,
};


/// Satellite delivery system descriptor.
///
/// EN 300 468 - 6.2.13.2
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc43 {
    #[bits(8, skip = 0x43)]
    #[bits(8, skip = 11)]

    /// Frequency in KHz.
    #[bits(32, from = Self::from_frequency, into = Self::into_frequency)]
    pub frequency: u32,

    /// Position in minutes of angle.
    #[bits(16, from = Self::from_orbital_position, into = Self::into_orbital_position)]
    pub orbital_position: u16,

    /// Satellite position in the western or eastern part of the orbit.
    #[bits(1)]
    pub west_east_flag: u8,

    /// Polarization of the transmitted signal.
    #[bits(2)]
    pub polarization: u8,

    /// Roll-off factor used in DVB-S2.
    #[bits(2)]
    pub rof: u8,

    /// Broadcast scheme used on a satellite delivery system.
    /// DVB-S2 or not.
    #[bits(1)]
    pub s2: u8,

    /// Modulation scheme used on a satellite delivery system.
    #[bits(2)]
    pub modulation: u8,

    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    #[bits(28, from = Self::from_symbol_rate, into = Self::into_symbol_rate)]
    pub symbol_rate: u32,

    /// Inner FEC scheme.
    #[bits(4)]
    pub fec: u8
}


impl Desc43 {
    #[inline]
    pub (crate) const fn size(&self) -> usize { 2 + 11 }

    #[inline]
    fn from_frequency(value: u32) -> u32 { value.from_bcd() * 10 }

    #[inline]
    fn into_frequency(value: u32) -> u32 { (value / 10).to_bcd() }

    #[inline]
    fn from_orbital_position(value: u16) -> u16 { value.from_bcd() * 6 }

    #[inline]
    fn into_orbital_position(value: u16) -> u16 { (value / 6).to_bcd() }

    #[inline]
    fn from_symbol_rate(value: u32) -> u32 { value.from_bcd() / 10 }

    #[inline]
    fn into_symbol_rate(value: u32) -> u32 { (value * 10).to_bcd() }
}


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::{
            constants,
            psi::Desc43,
        },
    };

    static DATA: &[u8] = &[
        0x43, 0x0b, 0x01, 0x23, 0x80, 0x00, 0x01, 0x30,
        0xa1, 0x02, 0x75, 0x00, 0x03,
    ];

    #[test]
    fn test_43_unpack() {
        let mut desc = Desc43::default();
        desc.unpack(DATA).unwrap();

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
    fn test_43_pack() {
        let desc = Desc43 {
            frequency: 12380000,
            orbital_position: 780,
            west_east_flag: constants::POSITION_EAST,
            polarization: constants::POLARIZATION_V,
            rof: 0,
            s2: 0,
            modulation: constants::MODULATION_DVB_S_QPSK,
            symbol_rate: 27500,
            fec: constants::FEC_3_4
        };

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
