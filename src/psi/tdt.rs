// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::{
        BCDTime,
        MJDFrom,
        MJDTo,
    },
};


/// TS Packet Identifier for TDT
pub const TDT_PID: u16 = 0x0014;


/// Time and Date Table carries only the UTC-time and date information
#[derive(Debug, Default, BitWrap)]
pub struct Tdt {
    #[bits(8,
        name = _table_id,
        value = 0x70,
        eq = 0x70)]

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(3, skip = 0b111)]
    #[bits(12,
        name = _section_length,
        value = 5,
        eq = 5)]

    /// Current time and date in UTC
    #[bits(40, from = Tdt::from_time, into = Tdt::into_time)]
    pub time: u64,
}


impl Tdt {
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
