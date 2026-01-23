use crate::{
    psi::{
        PsiSectionError,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
    },
};

/// TS Packet Identifier for TDT
pub const TDT_PID: u16 = 0x0014;

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
        if value.len() < 8 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != 0x70 {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        Ok(TdtSectionRef(&value[.. section_length]))
    }
}
