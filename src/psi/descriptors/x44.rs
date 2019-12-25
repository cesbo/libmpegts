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

use crate::psi::BCD;


/// Cable delivery system descriptor.
///
/// EN 300 468 - 6.2.13.1
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc44 {
    #[bits(8, skip = 0x44)]
    #[bits(8, skip = 0)]

    /// Frequency in Hz.
    #[bits(32, from = Self::from_frequency, into = Self::into_frequency)]
    pub frequency: u32,

    /// Outer FEC scheme.
    #[bits(12, skip = 0xFFF)]
    #[bits(4)]
    pub fec_outer: u8,

    /// Modulation scheme used on a cable delivery system.
    #[bits(8)]
    pub modulation: u8,

    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    #[bits(28, from = Self::from_symbol_rate, into = Self::into_symbol_rate)]
    pub symbol_rate: u32,

    /// Inner FEC scheme.
    #[bits(4)]
    pub fec: u8
}


impl std::convert::TryFrom<&[u8]> for Desc44 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc44 {
    #[inline]
    fn from_frequency(value: u32) -> u32 { value.from_bcd() * 100 }

    #[inline]
    fn into_frequency(value: u32) -> u32 { (value / 100).to_bcd() }

    #[inline]
    fn from_symbol_rate(value: u32) -> u32 { value.from_bcd() / 10 }

    #[inline]
    fn into_symbol_rate(value: u32) -> u32 { (value * 10).to_bcd() }

    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + 11 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        self.pack(&mut buffer[skip ..]).unwrap();
        buffer[skip + 1] = (size - 2) as u8;
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
        crate::{
            constants,
            psi::Desc44,
        },
    };

    static DATA: &[u8] = &[
        0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0,
        0x05, 0x00, 0x68, 0x75, 0x00,
    ];

    #[test]
    fn test_44_parse() {
        let desc = Desc44::try_from(DATA).unwrap();

        assert_eq!(desc.frequency, 346000000);
        assert_eq!(desc.fec_outer, constants::FEC_OUTER_NOT_DEFINED);
        assert_eq!(desc.modulation, constants::MODULATION_DVB_C_256_QAM);
        assert_eq!(desc.symbol_rate, 6875);
        assert_eq!(desc.fec, constants::FEC_NOT_DEFINED);
    }

    #[test]
    fn test_44_assemble() {
        let desc = Desc44 {
            frequency: 346000000,
            fec_outer: constants::FEC_OUTER_NOT_DEFINED,
            modulation: constants::MODULATION_DVB_C_256_QAM,
            symbol_rate: 6875,
            fec: constants::FEC_NOT_DEFINED
        };

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
