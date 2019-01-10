use crate::bytes::*;
use crate::bcd::*;
use crate::mjd::*;
use crate::psi::{Psi, PsiDemux, Descriptors};

/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;

/// Time and Date Table carries only the UTC-time and date information
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
        psi.buffer[0] == 0x73
    }

    /// Reads PSI packet and append data into the `Tot`
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
}
