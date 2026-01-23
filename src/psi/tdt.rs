use crate::{
    pack_bits,
    psi::{
        Psi,
        PsiDemux,
        PsiSectionError,
    },
    utils::{
        BcdTime,
        MjdFrom,
        MjdTo,
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

impl PsiDemux for Tdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x70);
        psi.buffer[1 .. 3].copy_from_slice(&pack_bits!(u16,
            section_syntax_indicator: 1 => 0,
            reserved_future_use: 1 => 0b1,
            reserved: 2 => 0b11,
            section_length: 12 => 5,
        ));

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

pub struct TdtSectionRef<'a>(&'a [u8]);

impl<'a> TdtSectionRef<'a> {
    /// Current time and date in UTC
    pub fn time(&self) -> u64 {
        u64::from_mjd([self.0[3], self.0[4]])
            + u32::from_bcd_time([self.0[5], self.0[6], self.0[7]]) as u64
    }
}

impl<'a> TryFrom<&'a [u8]> for TdtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(PsiSectionError::InvalidLength);
        }

        if value[0] != 0x70 {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = 3 + (u16::from_be_bytes([value[1], value[2]]) & 0x03FF) as usize;
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidLength);
        }

        Ok(TdtSectionRef(value))
    }
}
