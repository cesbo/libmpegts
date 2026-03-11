use crate::{
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
        crc32b,
    },
};

pub const EIT_PID: u16 = 0x0012;

pub struct EitEventRef<'a>(&'a [u8]);

impl<'a> EitEventRef<'a> {
    /// Event identification number
    pub fn event_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// Event start time in UTC
    pub fn start_time(&self) -> u64 {
        u64::from_mjd([self.0[2], self.0[3]])
            + u32::from_bcd_time([self.0[4], self.0[5], self.0[6]]) as u64
    }

    /// Event duration in seconds
    pub fn duration(&self) -> u32 {
        u32::from_bcd_time([self.0[7], self.0[8], self.0[9]])
    }

    /// Indicating the status of the event
    /// * `0` - undefined
    /// * `1` - not running
    /// * `2` - starts in a few seconds (e.g. for video recording)
    /// * `3` - pausing
    /// * `4` - running
    /// * `5` - service off-air
    pub fn running_status(&self) -> u8 {
        (self.0[10] & 0xe0) >> 5
    }

    /// On `true` indicates that access is controlled by a CA system
    pub fn free_ca_mode(&self) -> bool {
        (self.0[10] & 0x10) != 0
    }

    /// Program element descriptors
    pub fn event_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > 12).then(|| self.0[12 ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for EitEventRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 12 {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[10], value[11]]) & 0x0fff) as usize;
        let item_length = 12 + desc_length;
        if value.len() >= item_length {
            Ok(EitEventRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct EitEventIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for EitEventIter<'a> {
    type Item = Result<EitEventRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match EitEventRef::try_from(remaining) {
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

/// Event Information Table provides information in chronological order
/// regarding the events contained within each service.
pub struct EitSectionRef<'a>(&'a [u8]);

impl<'a> EitSectionRef<'a> {
    /// Table ID
    /// * `0x4e` - actual TS, present/following event information
    /// * `0x4f` - other TS, present/following event information
    /// * `0x50 ..= 0x5f` - actual TS, event schedule information
    /// * `0x60 ..= 0x6f` - other TS, event schedule information
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// EIT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Program number
    pub fn service_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// Transport Stream Identifier
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]])
    }

    /// Original Network ID
    pub fn original_network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[10], self.0[11]])
    }

    /// Iterator for EIT events
    pub fn events(&self) -> EitEventIter<'a> {
        let items_start = 14;
        let items_end = self.0.len() - 4; // Exclude CRC32
        EitEventIter {
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

impl<'a> TryFrom<&'a [u8]> for EitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 18 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            0x4e ..= 0x6f => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let pmt = EitSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - 4]);
        if checksum != pmt.crc32() {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(pmt)
    }
}

impl<'a> TryFrom<&'a Psi> for EitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => EitSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
