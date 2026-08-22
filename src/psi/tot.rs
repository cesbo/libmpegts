use crate::{
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        check_crc32,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
    },
};

/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;

/// Time Offset Table carries the UTC-time and date information and local time offset
pub struct TotSectionRef<'a>(&'a [u8]);

impl<'a> TotSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// Current time and date in UTC
    pub fn time(&self) -> u64 {
        u64::from_mjd([self.0[3], self.0[4]])
            + u32::from_bcd_time([self.0[5], self.0[6], self.0[7]]) as u64
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[8], self.0[9]]) & 0x0fff) as usize
    }

    /// List of descriptors.
    pub fn descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        (descriptors_len > 0).then(|| self.0[10 .. 10 + descriptors_len].into())
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - 4 ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for TotSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 14 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != 0x73 {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(TotSectionRef(&value[.. section_length]))
    }
}

impl<'a> TryFrom<&'a Psi> for TotSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => TotSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
