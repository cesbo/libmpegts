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
        Psi,
        PsiDemux,
    },
};


/// TS Packet Identifier for TDT
pub const TDT_PID: u16 = 0x0014;


/// Time and Date Table carries only the UTC-time and date information
#[derive(Debug, BitWrap)]
pub struct Tdt {
    #[bits(8)]
    pub table_id: u8,

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(3, skip = 0b111)]
    #[bits(12)]
    section_length: u16,

    /// Current time and date in UTC
    #[bits(40, from = Tdt::from_time, into = Tdt::into_time)]
    pub time: u64,
}


impl Default for Tdt {
    fn default() -> Self {
        Tdt {
            table_id: 0x70,
            section_syntax_indicator: 0,
            section_length: 5,
            time: 0,
        }
    }
}


impl Tdt {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size == 8 &&
        psi.buffer[0] == 0x70
    }

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

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.unpack(&psi.buffer).unwrap();
    }
}


impl PsiDemux for Tdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::default();

        psi.buffer.resize(psi.buffer.len() + 8, 0);
        self.pack(&mut psi.buffer).unwrap();

        vec![psi]
    }

    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        let mut psi = psi_list.first_mut().unwrap();
        psi.pid = pid;
        psi.cc = *cc;
        psi.size = psi.buffer.len();
        psi.demux(dst);
        *cc = psi.cc;
    }
}


impl From<&Psi> for Tdt {
    fn from(psi: &Psi) -> Self {
        let mut tdt = Tdt::default();
        tdt.parse(psi);
        tdt
    }
}
