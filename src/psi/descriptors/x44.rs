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
    #[bits_skip(8, 0x44)]
    #[bits_skip(8, 0)]

    /// Frequency in Hz.
    #[bits_convert(32, Self::from_frequency, Self::into_frequency)]
    pub frequency: u32,

    /// Outer FEC scheme.
    #[bits_skip(12, 0xFFF)]
    #[bits(4)]
    pub fec_outer: u8,

    /// Modulation scheme used on a cable delivery system.
    #[bits(8)]
    pub modulation: u8,

    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    #[bits_convert(28, Self::from_symbol_rate, Self::into_symbol_rate)]
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
    use bitwrap::BitWrap;

    use crate::{
        constants,
        psi::{
            Descriptor,
            Descriptors,
            Desc44,
        },
    };

    static DATA_44: &[u8] = &[
        0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0,
        0x05, 0x00, 0x68, 0x75, 0x00,
    ];

    #[test]
    fn test_44_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.unpack(DATA_44).unwrap();

        let mut iter = descriptors.iter();
        if let Some(Descriptor::Desc44(desc)) = iter.next() {
            assert_eq!(desc.frequency, 346000000);
            assert_eq!(desc.fec_outer, constants::FEC_OUTER_NOT_DEFINED);
            assert_eq!(desc.modulation, constants::MODULATION_DVB_C_256_QAM);
            assert_eq!(desc.symbol_rate, 6875);
            assert_eq!(desc.fec, constants::FEC_NOT_DEFINED);
        } else {
            unreachable!();
        }
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
