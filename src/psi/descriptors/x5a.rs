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


/// Terrestrial delivery system descriptor.
///
/// EN 300 468 - 6.2.13.4
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc5A {
    #[bits_skip(8, 0x5A)]
    #[bits_skip(8, 0)]

    /// Frequency in Hz.
    #[bits_convert(32, Self::from_frequency, Self::into_frequency)]
    pub frequency: u32,

    /// Used bandwidth.
    #[bits(3)]
    pub bandwidth: u8,

    /// Stream's hierarchical priority.
    /// * `1`  - associated TS is a HP (high priority) stream
    /// * `0` - associated TS is a LP (low priority) stream
    #[bits(1)]
    pub priority: u8,

    /// Usage of time slicing.
    /// * `1`  - Time Slicing is not used.
    /// * `0` - at least one elementary stream uses Time Slicing
    #[bits(1)]
    pub time_slicing: u8,

    /// Usage of the MPE-FEC.
    /// * `1`  - MPE-FEC is not used
    /// * `0` - at least one elementary stream uses MPE-FEC
    #[bits(1)]
    pub mpe_fec: u8,

    /// Modulation scheme used on a terrestrial delivery system.
    #[bits_skip(2, 0b11)]
    #[bits(2)]
    pub modulation: u8,

    /// Specifies whether the transmission is hierarchical and,
    /// if so, what the α value is.
    #[bits(3)]
    pub hierarchy: u8,

    /// HP stream inner FEC scheme.
    #[bits(3)]
    pub code_rate_hp: u8,

    /// LP stream inner FEC scheme.
    #[bits(3)]
    pub code_rate_lp: u8,

    /// Guard interval value.
    #[bits(2)]
    pub guard_interval: u8,

    /// Number of carriers in an OFDM frame.
    #[bits(2)]
    pub transmission: u8,

    /// Indicates whether other frequencies are in use.
    /// * `1`  - one or more other frequencies are in use
    /// * `0` - no other frequency is in use
    #[bits(1)]
    #[bits_skip(32, 0xFFFF_FFFF)]
    pub other_frequency_flag: u8
}


impl std::convert::TryFrom<&[u8]> for Desc5A {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc5A {
    #[inline]
    fn from_frequency(value: u32) -> u32 { value * 10 }

    #[inline]
    fn into_frequency(value: u32) -> u32 { value / 10 }

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
            psi::Desc5A,
        },
    };

    static DATA: &[u8] = &[
        0x5a, 0x0b, 0x02, 0xfa, 0xf0, 0x80, 0x1f, 0x81,
        0x1a, 0xff, 0xff, 0xff, 0xff,
    ];

    #[test]
    fn test_5a_parse() {
        let desc = Desc5A::try_from(DATA).unwrap();

        assert_eq!(desc.frequency, 500000000);
        assert_eq!(desc.bandwidth, constants::BANDWIDTH_DVB_T_8MHZ);
        assert_eq!(desc.priority, 1);
        assert_eq!(desc.time_slicing, 1);
        assert_eq!(desc.mpe_fec, 1);
        assert_eq!(desc.modulation, constants::MODULATION_DVB_T_64QAM);
        assert_eq!(desc.hierarchy, constants::HIERARCHY_DVB_T_NON_NATIVE);
        assert_eq!(desc.code_rate_hp, constants::CODE_RATE_DVB_T_2_3);
        assert_eq!(desc.code_rate_lp, 0);
        assert_eq!(desc.guard_interval, constants::GUARD_INTERVAL_1_4);
        assert_eq!(desc.transmission, constants::TRANSMISSION_MODE_8K);
        assert_eq!(desc.other_frequency_flag, 0);
    }

    #[test]
    fn test_5a_assemble() {
        let desc = Desc5A {
            frequency: 500000000,
            bandwidth: constants::BANDWIDTH_DVB_T_8MHZ,
            priority: 1,
            time_slicing: 1,
            mpe_fec: 1,
            modulation: constants::MODULATION_DVB_T_64QAM,
            hierarchy: constants::HIERARCHY_DVB_T_NON_NATIVE,
            code_rate_hp: constants::CODE_RATE_DVB_T_2_3,
            code_rate_lp: 0,
            guard_interval: constants::GUARD_INTERVAL_1_4,
            transmission: constants::TRANSMISSION_MODE_8K,
            other_frequency_flag: 0
        };

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
