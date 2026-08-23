use crate::{
    pack_bits,
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        PsiSectionMut,
        Sections,
        check_crc32,
        finalize_sections,
        psi_section_length,
    },
    ts::PID_NONE,
};

const PMT_TABLE_ID: u8 = 0x02;
const PMT_HEADER_SIZE: usize = 12;
const PMT_ITEM_HEADER_SIZE: usize = 5;
const PMT_CRC_SIZE: usize = 4;
const PMT_SECTION_SIZE: usize = 1024;

pub struct PmtStreamRef<'a>(&'a [u8]);

impl<'a> PmtStreamRef<'a> {
    /// Type of program element
    pub fn stream_type(&self) -> u8 {
        self.0[0]
    }

    /// TS Packet Identifier
    pub fn elementary_pid(&self) -> u16 {
        u16::from_be_bytes([self.0[1], self.0[2]]) & 0x1fff
    }

    /// Program element descriptors
    pub fn stream_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > 5).then(|| self.0[5 ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for PmtStreamRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < PMT_ITEM_HEADER_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let es_info_length = (u16::from_be_bytes([value[3], value[4]]) & 0x0fff) as usize;
        let item_length = PMT_ITEM_HEADER_SIZE + es_info_length;
        if value.len() >= item_length {
            Ok(PmtStreamRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct PmtStreamIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for PmtStreamIter<'a> {
    type Item = Result<PmtStreamRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match PmtStreamRef::try_from(remaining) {
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
    pub fn program_number(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// PCR (Program Clock Reference) pid.
    pub fn pcr_pid(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]]) & 0x1fff
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[10], self.0[11]]) & 0x0fff) as usize
    }

    /// List of descriptors.
    pub fn program_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        let end = PMT_HEADER_SIZE + descriptors_len;
        (descriptors_len > 0).then(|| self.0[PMT_HEADER_SIZE .. end].into())
    }

    /// Iterator for PMT streams
    pub fn streams(&self) -> PmtStreamIter<'a> {
        let descriptors_len = self.descriptors_length();
        let items_start = PMT_HEADER_SIZE + descriptors_len;
        let items_end = self.0.len() - PMT_CRC_SIZE; // Exclude CRC32
        PmtStreamIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - PMT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for PmtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < PMT_HEADER_SIZE + PMT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != PMT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(PmtSectionRef(&value[.. section_length]))
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

/// Elementary stream entry for [`PmtConfig`].
#[derive(Clone)]
pub struct PmtStream {
    // MPEG-TS stream type (e.g. 0x1B for H.264)
    pub stream_type: u8,
    // PID to assign to this stream (must be unique and >= 0x20 and < 0x1FFF)
    pub elementary_pid: u16,
    // Raw ES-level descriptor bytes for PMT
    pub stream_descriptors: Vec<u8>,
}

/// Input configuration for [`PmtBuilder`].
pub struct PmtConfig {
    pub program_number: u16,
    pub pcr_pid: u16,
    pub version: u8,
    pub program_descriptors: Vec<u8>,
    pub streams: Vec<PmtStream>,
}

/// Builder for PMT (Program Map Table) sections.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{PmtBuilder, PmtConfig, PmtSectionRef, PmtStream};
///
/// let sections = PmtBuilder::build(PmtConfig {
///     program_number: 1,
///     pcr_pid: 256,
///     version: 0,
///     program_descriptors: Vec::new(),
///     streams: vec![
///         PmtStream {
///             stream_type: 2,
///             elementary_pid: 257,
///             stream_descriptors: Vec::new(),
///         },
///         PmtStream {
///             stream_type: 4,
///             elementary_pid: 258,
///             stream_descriptors: Vec::new(),
///         },
///     ],
/// });
/// assert_eq!(sections.len(), 1);
/// let pmt = PmtSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(pmt.program_number(), 1);
/// ```
pub struct PmtBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    program_number: u16,
    pcr_pid: u16,
    version: u8,
    program_descriptors: Vec<u8>,
}

impl PmtBuilder {
    /// Converts a PMT config into finalized PSI sections.
    pub fn build(config: PmtConfig) -> Sections {
        let mut builder = Self {
            buffer: Vec::with_capacity(PMT_SECTION_SIZE),
            starts: Vec::new(),
            program_number: config.program_number,
            pcr_pid: config.pcr_pid,
            version: config.version & 0x1f,
            program_descriptors: config.program_descriptors,
        };

        for stream in config.streams {
            builder.push(stream);
        }

        builder.finalize()
    }

    /// Adds an elementary stream to the current section.
    fn push(&mut self, stream: PmtStream) {
        debug_assert!(stream.elementary_pid < PID_NONE);

        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let last_section_start = *self.starts.last().unwrap();
            let current_section_size = self.buffer.len() - last_section_start;
            let item_size = PMT_ITEM_HEADER_SIZE + stream.stream_descriptors.len();
            if current_section_size + item_size + PMT_CRC_SIZE > PMT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.push(stream.stream_type);
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => stream.elementary_pid,
        ));
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 4 => 0b1111,
            es_info_length: 12 => stream.stream_descriptors.len() as u16,
        ));
        self.buffer.extend_from_slice(&stream.stream_descriptors);
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    /// Consumes the builder and returns a [`Sections`] collection.
    fn finalize(mut self) -> Sections {
        if self.starts.is_empty() {
            self.begin_section();
        }

        self.seal_section();

        finalize_sections(self.buffer, self.starts)
    }

    /// Writes the 12-byte section header and program descriptors.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());

        // Bytes 0-7: table_id, section_length, program_number, version, section/last_section
        self.buffer.extend_from_slice(&pack_bits!(u64,
            table_id: 8 => PMT_TABLE_ID,
            section_syntax_indicator: 1 => 1,
            private_bit: 1 => 0,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            program_number: 16 => self.program_number,
            reserved2: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));

        // Bytes 8-11: PCR_PID + program_info_length
        let program_info_length = self.program_descriptors.len() as u16;
        self.buffer.extend_from_slice(&pack_bits!(u32,
            reserved: 3 => 0b111,
            pcr_pid: 13 => self.pcr_pid,
            reserved2: 4 => 0b1111,
            program_info_length: 12 => program_info_length,
        ));

        // Program-level descriptors
        if !self.program_descriptors.is_empty() {
            self.buffer.extend_from_slice(&self.program_descriptors);
        }
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; PMT_CRC_SIZE]);
    }
}

/// Mutable borrowed PMT section for in-place patching.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{PmtBuilder, PmtConfig, PmtSectionMut, PmtSectionRef};
///
/// let sections = PmtBuilder::build(PmtConfig {
///     program_number: 1,
///     pcr_pid: 0x1fff,
///     version: 0,
///     program_descriptors: Vec::new(),
///     streams: Vec::new(),
/// });
/// let mut section = sections[0].to_vec();
///
/// let mut pmt = PmtSectionMut::try_from(&mut section[..]).unwrap();
/// pmt.set_pcr_pid(256);
/// pmt.set_version(1);
/// pmt.update_crc32();
///
/// let pmt = PmtSectionRef::try_from(&section[..]).unwrap();
/// assert_eq!(pmt.pcr_pid(), 256);
/// assert_eq!(pmt.version(), 1);
/// ```
pub struct PmtSectionMut<'a>(PsiSectionMut<'a>);

impl<'a> PmtSectionMut<'a> {
    /// Sets the 13-bit PCR PID, preserving the reserved bits.
    pub fn set_pcr_pid(&mut self, pid: u16) {
        let section = self.0.as_mut();
        let pid = pid & 0x1fff;
        section[8] = (section[8] & 0xe0) | ((pid >> 8) as u8);
        section[9] = pid as u8;
    }

    /// Sets the 5-bit `version_number`, preserving the reserved bits and
    /// `current_next_indicator`.
    pub fn set_version(&mut self, version: u8) {
        self.0.set_version(version);
    }

    /// Recomputes the CRC32 over the section body and writes it into the
    /// last four bytes.
    pub fn update_crc32(&mut self) {
        self.0.update_crc32();
    }
}

impl<'a> TryFrom<&'a mut [u8]> for PmtSectionMut<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a mut [u8]) -> Result<Self, Self::Error> {
        if value.len() < PMT_HEADER_SIZE + PMT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != PMT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        Ok(Self(PsiSectionMut::try_from(value)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pmt_empty() {
        let sections = PmtBuilder::build(PmtConfig {
            program_number: 1,
            pcr_pid: 256,
            version: 0,
            program_descriptors: Vec::new(),
            streams: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].len(), 16); // header(12) + CRC(4)

        let pmt = PmtSectionRef::try_from(&sections[0][..]).expect("Valid empty PMT");
        assert_eq!(pmt.program_number(), 1);
        assert_eq!(pmt.pcr_pid(), 256);
        assert_eq!(pmt.version(), 0);
        assert!(pmt.program_descriptors().is_none());
        assert_eq!(pmt.streams().count(), 0);
    }

    #[test]
    fn test_build_pmt_with_descriptors() {
        let program_descriptors: &[u8] = &[0x09, 0x04, 0x01, 0x02, 0x03, 0x04];

        let sections = PmtBuilder::build(PmtConfig {
            program_number: 100,
            pcr_pid: 512,
            version: 3,
            program_descriptors: program_descriptors.to_vec(),
            streams: vec![PmtStream {
                stream_type: 2,
                elementary_pid: 513,
                stream_descriptors: Vec::new(),
            }],
        });

        assert_eq!(sections.len(), 1);

        let pmt = PmtSectionRef::try_from(&sections[0][..]).expect("Valid PMT");
        assert_eq!(pmt.program_number(), 100);
        assert_eq!(pmt.pcr_pid(), 512);
        assert_eq!(pmt.version(), 3);

        let descriptors = pmt.program_descriptors().expect("Program descriptors");
        let desc = descriptors.into_iter().next().unwrap().unwrap();
        assert_eq!(desc.tag(), 0x09);

        let mut streams = pmt.streams();
        let stream = streams.next().expect("First stream").expect("Valid stream");
        assert_eq!(stream.stream_type(), 2);
        assert_eq!(stream.elementary_pid(), 513);
        assert!(stream.stream_descriptors().is_none());
        assert!(streams.next().is_none());
    }

    #[test]
    fn test_build_pmt_sections_index() {
        let sections = PmtBuilder::build(PmtConfig {
            program_number: 1,
            pcr_pid: 100,
            version: 0,
            program_descriptors: Vec::new(),
            streams: vec![PmtStream {
                stream_type: 2,
                elementary_pid: 101,
                stream_descriptors: Vec::new(),
            }],
        });

        assert_eq!(sections.len(), 1);
        PmtSectionRef::try_from(&sections[0][..]).expect("Valid section");
    }

    #[test]
    fn test_pmt_section_mut() {
        let sections = PmtBuilder::build(PmtConfig {
            program_number: 1,
            pcr_pid: 0x1fff,
            version: 3,
            program_descriptors: Vec::new(),
            streams: vec![
                PmtStream {
                    stream_type: 0x1b,
                    elementary_pid: 256,
                    stream_descriptors: Vec::new(),
                },
                PmtStream {
                    stream_type: 0x0f,
                    elementary_pid: 257,
                    stream_descriptors: Vec::new(),
                },
            ],
        });
        let original = sections[0].to_vec();

        let mut section = original.clone();
        let mut pmt = PmtSectionMut::try_from(&mut section[..]).expect("valid section");
        pmt.set_pcr_pid(256);
        pmt.set_version(4);
        pmt.update_crc32();
        assert_eq!(section.len(), original.len(), "section length unchanged");

        let pmt = PmtSectionRef::try_from(&section[..]).expect("CRC32 valid after patch");
        assert_eq!(pmt.pcr_pid(), 256);
        assert_eq!(pmt.version(), 4);
        assert_eq!(pmt.program_number(), 1);

        // only pcr_pid, version and CRC32 bytes may differ
        let crc_offset = section.len() - 4;
        for (i, (a, b)) in original.iter().zip(section.iter()).enumerate() {
            if i == 5 || i == 8 || i == 9 || i >= crc_offset {
                continue;
            }
            assert_eq!(a, b, "byte {i} must be unchanged");
        }

        // reserved bits and current_next_indicator preserved
        assert_eq!(section[8] & 0xe0, 0xe0);
        assert_eq!(section[5] & 0xc1, original[5] & 0xc1);
    }

    #[test]
    fn test_pmt_section_mut_invalid() {
        let sections = PmtBuilder::build(PmtConfig {
            program_number: 1,
            pcr_pid: 0x1fff,
            version: 0,
            program_descriptors: Vec::new(),
            streams: vec![PmtStream {
                stream_type: 0x1b,
                elementary_pid: 256,
                stream_descriptors: Vec::new(),
            }],
        });
        let original = sections[0].to_vec();

        // wrong table_id
        let mut section = original.clone();
        section[0] = 0x03;
        let copy = section.clone();
        assert!(matches!(
            PmtSectionMut::try_from(&mut section[..]),
            Err(PsiSectionError::InvalidTableId)
        ));
        assert_eq!(section, copy, "section unchanged on error");

        // truncated slice: declared section length no longer matches
        let mut section = original.clone();
        let short = section.len() - 1;
        let copy = section.clone();
        assert!(matches!(
            PmtSectionMut::try_from(&mut section[.. short]),
            Err(PsiSectionError::InvalidSectionLength)
        ));
        assert_eq!(section, copy, "section unchanged on error");

        // shorter than the fixed header plus CRC32
        let mut section = [0x02u8; 10];
        assert!(matches!(
            PmtSectionMut::try_from(&mut section[..]),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }
}
