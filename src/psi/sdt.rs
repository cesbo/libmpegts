use crate::{
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        psi_section_length,
    },
    utils::crc32b,
};

pub const SDT_PID: u16 = 0x0011;

pub struct SdtServiceRef<'a>(&'a [u8]);

impl<'a> SdtServiceRef<'a> {
    /// Program number.
    pub fn service_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// Indicates that EIT schedule information for the service is present in the current TS.
    pub fn eit_schedule_flag(&self) -> bool {
        (self.0[2] & 0x02) != 0
    }

    /// Indicates that EIT_present_following information for the service is present in the current TS.
    pub fn eit_present_following_flag(&self) -> bool {
        (self.0[2] & 0x01) != 0
    }

    /// Indicating the status of the service.
    pub fn running_status(&self) -> u8 {
        (self.0[3] & 0xe0) >> 5
    }

    /// On `true` indicates that access is controlled by a CA system
    pub fn free_ca_mode(&self) -> bool {
        (self.0[3] & 0x10) != 0
    }

    /// Service descriptors
    pub fn service_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > 5).then(|| self.0[5 ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for SdtServiceRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 5 {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[3], value[4]]) & 0x0fff) as usize;
        let item_length = 5 + desc_length;
        if value.len() >= item_length {
            Ok(SdtServiceRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct SdtServiceIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for SdtServiceIter<'a> {
    type Item = Result<SdtServiceRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match SdtServiceRef::try_from(remaining) {
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

/// Service Description Table - contains data describing the services
/// in the system e.g. names of services, the service provider, etc.
///
/// EN 300 468 - 5.2.3
pub struct SdtSectionRef<'a>(&'a [u8]);

impl<'a> SdtSectionRef<'a> {
    /// Table ID
    /// * `0x42` - actual TS
    /// * `0x46` - other TS
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// SDT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Transport Stream Identifier
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// Original Network ID
    pub fn original_network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]])
    }

    /// Iterator for SDT services
    pub fn services(&self) -> SdtServiceIter<'a> {
        let items_start = 11;
        let items_end = self.0.len() - 4; // Exclude CRC32
        SdtServiceIter {
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

impl<'a> TryFrom<&'a [u8]> for SdtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 15 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            0x42 | 0x46 => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let sdt = SdtSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - 4]);
        if checksum != sdt.crc32() {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(sdt)
    }
}

impl<'a> TryFrom<&'a Psi> for SdtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => SdtSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
