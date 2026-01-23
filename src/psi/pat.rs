/// Program Association Table (PAT) implementation
use crate::{
    psi::{
        Psi,
        PsiSectionError,
        psi_section_length,
    },
    utils::crc32b,
};

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;

pub struct PatItemRef<'a>(&'a [u8]);

impl<'a> PatItemRef<'a> {
    /// Program Number
    pub fn pnr(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// TS Packet Identifier
    pub fn pid(&self) -> u16 {
        u16::from_be_bytes([self.0[2], self.0[3]]) & 0x1fff
    }
}

impl<'a> TryFrom<&'a [u8]> for PatItemRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            Err(PsiSectionError::InvalidSectionLength)
        } else {
            Ok(PatItemRef(&value[0 .. 4]))
        }
    }
}

/// Program Association Table provides the correspondence between a `pnr` (Program Number) and
/// the `pid` value of the TS packets which carry the program definition.
pub struct PatSectionRef<'a>(&'a [u8]);

impl<'a> PatSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// Transport Stream ID to identify actual stream from any other multiplex within a network
    pub fn tsid(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// PAT version
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Iterator for PAT Items
    pub fn items(&self) -> impl Iterator<Item = Result<PatItemRef<'a>, PsiSectionError>> {
        let ptr = &self.0[8 .. self.0.len() - 4];
        ptr.chunks(4).map(PatItemRef::try_from)
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - 4 ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for PatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 8 + 4 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != 0x00 {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let pat = PatSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - 4]);
        if checksum != pat.crc32() {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(pat)
    }
}

impl<'a> TryFrom<&'a Psi> for PatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => PatSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
