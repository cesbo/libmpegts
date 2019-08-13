use crate::{
    bytes::*,
    psi::{
        BCDTime,
        MJDFrom,
        MJDTo,
        Psi,
        PsiDemux,
        Descriptors,
    },
};


/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;


/// Time Offset Table carries the UTC-time and date information and local time offset
#[derive(Default, Debug)]
pub struct Tot {
    /// Current time and date in UTC
    pub time: u64,
    /// List of descriptors.
    pub descriptors: Descriptors
}


impl Tot {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 10 + 4 &&
        psi.buffer[0] == 0x73 &&
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.time = psi.buffer[3 ..].get_u16().from_mjd() +
            u64::from(psi.buffer[5 ..].get_u24().from_bcd_time());

        let descriptors_len = (psi.buffer[8 ..].get_u16() & 0x0FFF) as usize;
        self.descriptors.parse(&psi.buffer[10 .. 10 + descriptors_len]);
    }
}


impl PsiDemux for Tot {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x73, 10, 0);
        psi.buffer[1] = 0x70; /* reserved bits */

        psi.buffer.resize(10, 0x00);
        psi.buffer[3 ..].set_u16(self.time.to_mjd());
        psi.buffer[5 ..].set_u24((self.time as u32).to_bcd_time());

        let descriptors_len = self.descriptors.assemble(&mut psi.buffer) as u16;
        psi.buffer[8 ..].set_u16(0xF000 | descriptors_len);

        vec![psi]
    }

    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        let mut psi = psi_list.first_mut().unwrap();
        psi.finalize();
        psi.pid = pid;
        psi.cc = *cc;
        psi.size = psi.buffer.len();
        psi.demux(dst);
        *cc = psi.cc;
    }
}
