use crate::psi::{
    BcdTime,
    Descriptors,
    MjdFrom,
    MjdTo,
    Psi,
    PsiDemux,
};

/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;

/// Time Offset Table carries the UTC-time and date information and local time offset
#[derive(Default, Debug)]
pub struct Tot {
    /// Current time and date in UTC
    pub time: u64,
    /// List of descriptors.
    pub descriptors: Descriptors,
}

impl Tot {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 10 + 4 && psi.buffer[0] == 0x73 && psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.time = u64::from_mjd([psi.buffer[3], psi.buffer[4]])
            + u32::from_bcd_time([psi.buffer[5], psi.buffer[6], psi.buffer[7]]) as u64;

        let descriptors_len =
            (u16::from_be_bytes([psi.buffer[8], psi.buffer[9]]) & 0x0FFF) as usize;
        self.descriptors
            .parse(&psi.buffer[10 .. 10 + descriptors_len]);
    }
}

impl PsiDemux for Tot {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x73, 3, 0);
        psi.buffer[1] = 0x70; /* reserved bits */

        psi.buffer.extend_from_slice(&self.time.into_mjd());
        psi.buffer.extend_from_slice(&self.time.into_bcd_time());

        let skip = psi.buffer.len();
        psi.buffer.extend_from_slice(&[0x00, 0x00]); /* descriptors_length placeholder */

        let descriptors_len = self.descriptors.assemble(&mut psi.buffer) as u16;
        psi.buffer[skip .. skip + 2].copy_from_slice(&(0xF000 | descriptors_len).to_be_bytes());

        vec![psi]
    }

    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        let psi = psi_list.first_mut().unwrap();
        psi.finalize();
        psi.pid = pid;
        psi.cc = *cc;
        psi.size = psi.buffer.len();
        psi.demux(dst);
        *cc = psi.cc;
    }
}

impl From<&Psi> for Tot {
    fn from(psi: &Psi) -> Self {
        let mut tot = Tot::default();
        tot.parse(psi);
        tot
    }
}
