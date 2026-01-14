use crate::psi::{
    BcdTime,
    MjdFrom,
    MjdTo,
    Psi,
    PsiDemux,
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
        psi.size == 8 && psi.buffer[0] == 0x70
    }

    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.time = u64::from_mjd([psi.buffer[3], psi.buffer[4]])
            + u32::from_bcd_time([psi.buffer[5], psi.buffer[6], psi.buffer[7]]) as u64;
    }
}

impl PsiDemux for Tdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x70, 3, 0);
        psi.buffer[1] = 0x70; /* reserved bits */
        psi.buffer[2] = 5;

        psi.buffer.extend_from_slice(&self.time.into_mjd());
        psi.buffer.extend_from_slice(&self.time.into_bcd_time());

        vec![psi]
    }

    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        let psi = psi_list.first_mut().unwrap();
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
