// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::{
        Tdt,
        Descriptor,
    },
};


/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;


/// Time Offset Table carries the UTC-time and date information and local time offset
#[derive(Default, Debug, BitWrap)]
pub struct Tot {
    #[bits(8, skip = 0x73)]

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(1, skip = 0b1)]
    #[bits(2, skip = 0b11)]
    #[bits(12,
        name = section_length,
        value = self.size() - 3)]

    /// Current time and date in UTC
    #[bits(40, from = Tdt::from_time, into = Tdt::into_time)]
    pub time: u64,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = descriptors_length,
        value = section_length - 7 - 4)]

    /// List of descriptors.
    #[bytes(descriptors_length)]
    pub descriptors: Vec<Descriptor>,
}


impl Tot {
    #[inline]
    fn descriptors_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn size(&self) -> usize {
        10 +
        self.descriptors_length() +
        4
    }
}
