// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::{
    bytes::*,
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
#[derive(Default, Debug)]
pub struct Tdt {
    /// Current time and date in UTC
    pub time: u64,
}


impl Tdt {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size == 8 &&
        psi.buffer[0] == 0x70
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.time = psi.buffer[3 ..].get_u16().from_mjd() +
            u64::from(psi.buffer[5 ..].get_u24().from_bcd_time());
    }
}


impl PsiDemux for Tdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x70, 3, 0);
        psi.buffer[1] = 0x70; /* reserved bits */
        psi.buffer[2] = 5;

        psi.buffer.resize(8, 0x00);
        psi.buffer[3 ..].set_u16(self.time.to_mjd());
        psi.buffer[5 ..].set_u24((self.time as u32).to_bcd_time());

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
