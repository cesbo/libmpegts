use crate::{
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        psi_section_length,
    },
    utils::crc32b,
};

pub struct PmtItemRef<'a>(&'a [u8]);

impl<'a> PmtItemRef<'a> {
    /// Type of program element
    pub fn stream_type(&self) -> u8 {
        self.0[0]
    }

    /// TS Packet Identifier
    pub fn pid(&self) -> u16 {
        u16::from_be_bytes([self.0[1], self.0[2]]) & 0x1fff
    }

    /// Program element descriptors
    pub fn descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > 5).then(|| self.0[5 ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for PmtItemRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 5 {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let es_info_length = (u16::from_be_bytes([value[3], value[4]]) & 0x0fff) as usize;
        let item_length = 5 + es_info_length;
        if value.len() >= item_length {
            Ok(PmtItemRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct PmtItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for PmtItemIter<'a> {
    type Item = Result<PmtItemRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match PmtItemRef::try_from(remaining) {
            Ok(item) => {
                self.offset += item.len();
                Some(Ok(item))
            }
            Err(e) => {
                self.offset = self.data.len(); // Stop iteration on error
                Some(Err(e))
            }
        }
    }
}

/// Program Map Table - provides the mappings between program numbers
/// and the program elements that comprise them.
pub struct PmtSectionRef<'a>(&'a [u8]);

impl<'a> PmtSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// PMT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Program number.
    pub fn pnr(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// PCR (Program Clock Reference) pid.
    pub fn pcr(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]]) & 0x1fff
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[10], self.0[11]]) & 0x0fff) as usize
    }

    /// List of descriptors.
    pub fn descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        (descriptors_len > 0).then(|| self.0[12 .. 12 + descriptors_len].into())
    }

    /// Iterator for PMT items
    pub fn items(&self) -> PmtItemIter<'a> {
        let descriptors_len = self.descriptors_length();
        let items_start = 12 + descriptors_len;
        let items_end = self.0.len() - 4; // Exclude CRC32
        PmtItemIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - 4 ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for PmtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 12 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != 0x02 {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let pmt = PmtSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - 4]);
        if checksum != pmt.crc32() {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(pmt)
    }
}

impl<'a> TryFrom<&'a Psi> for PmtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => PmtSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
