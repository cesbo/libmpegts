use crate::psi::{
    DescriptorsRef,
    Psi,
    PsiSectionError,
    check_crc32,
    psi_section_length,
};

pub const NIT_PID: u16 = 0x0010;

pub struct NitTransportStreamRef<'a>(&'a [u8]);

impl<'a> NitTransportStreamRef<'a> {
    /// Transport Stream Identifier
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// Original Network ID
    pub fn original_network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[2], self.0[3]])
    }

    /// Program element descriptors
    pub fn transport_stream_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > 6).then(|| self.0[6 ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for NitTransportStreamRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 6 {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[4], value[5]]) & 0x0fff) as usize;
        let item_length = 6 + desc_length;
        if value.len() >= item_length {
            Ok(NitTransportStreamRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct NitTransportStreamIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for NitTransportStreamIter<'a> {
    type Item = Result<NitTransportStreamRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match NitTransportStreamRef::try_from(remaining) {
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

/// The NIT conveys information relating to the physical organization
/// of the multiplexes/TSs carried via a given network,
/// and the characteristics of the network itself.
///
/// EN 300 468 - 5.2.1
pub struct NitSectionRef<'a>(&'a [u8]);

impl<'a> NitSectionRef<'a> {
    /// Table ID
    /// * `0x40` - actual network
    /// * `0x41` - other network
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// NIT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Network ID
    pub fn network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[8], self.0[9]]) & 0x0fff) as usize
    }

    /// List of descriptors.
    pub fn network_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        (descriptors_len > 0).then(|| self.0[12 .. 12 + descriptors_len].into())
    }

    /// Iterator for NIT transport streams
    pub fn transport_streams(&self) -> NitTransportStreamIter<'a> {
        let descriptors_len = self.descriptors_length();
        let items_start = 12 + descriptors_len;
        let items_end = self.0.len() - 4; // Exclude CRC32
        NitTransportStreamIter {
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

impl<'a> TryFrom<&'a [u8]> for NitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 16 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            0x40 | 0x41 => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(NitSectionRef(&value[.. section_length]))
    }
}

impl<'a> TryFrom<&'a Psi> for NitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => NitSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}
