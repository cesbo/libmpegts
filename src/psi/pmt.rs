use crate::{
    pack_bits,
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        Sections,
        psi_section_length,
    },
    ts::PID_NONE,
    utils::crc32b,
};

const PMT_TABLE_ID: u8 = 0x02;
const PMT_HEADER_SIZE: usize = 12;
const PMT_ITEM_HEADER_SIZE: usize = 5;
const PMT_CRC_SIZE: usize = 4;
const PMT_SECTION_SIZE: usize = 1024;

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
        if value.len() < PMT_ITEM_HEADER_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let es_info_length = (u16::from_be_bytes([value[3], value[4]]) & 0x0fff) as usize;
        let item_length = PMT_ITEM_HEADER_SIZE + es_info_length;
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
        let end = PMT_HEADER_SIZE + descriptors_len;
        (descriptors_len > 0).then(|| self.0[PMT_HEADER_SIZE .. end].into())
    }

    /// Iterator for PMT items
    pub fn items(&self) -> PmtItemIter<'a> {
        let descriptors_len = self.descriptors_length();
        let items_start = PMT_HEADER_SIZE + descriptors_len;
        let items_end = self.0.len() - PMT_CRC_SIZE; // Exclude CRC32
        PmtItemIter {
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

        let pmt = PmtSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - PMT_CRC_SIZE]);
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

/// Builder for PMT (Program Map Table) sections.
///
/// # Examples
///
/// ```
/// use mpegts::psi::{PmtBuilder, PmtSectionRef};
///
/// let mut builder = PmtBuilder::new(1, 256);
/// builder.push(2, 257, &[]);
/// builder.push(4, 258, &[]);
/// let sections = builder.finalize();
/// assert_eq!(sections.len(), 1);
/// let pmt = PmtSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(pmt.pnr(), 1);
/// ```
pub struct PmtBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    pnr: u16,
    pcr_pid: u16,
    version: u8,
    program_descriptors: Vec<u8>,
    finalized: bool,
}

impl PmtBuilder {
    /// Creates a new PMT builder.
    ///
    /// - `pnr` — Program Number
    /// - `pcr_pid` — PID of the TS packets carrying the PCR
    pub fn new(pnr: u16, pcr_pid: u16) -> Self {
        Self {
            buffer: Vec::with_capacity(PMT_SECTION_SIZE),
            starts: Vec::new(),
            pnr,
            pcr_pid,
            version: 0,
            program_descriptors: Vec::new(),
            finalized: false,
        }
    }

    /// Sets PMT version (0..31).
    pub fn set_version(&mut self, version: u8) {
        self.version = version & 0x1f;
    }

    /// Sets program-level descriptors (raw bytes).
    /// Must be called before any `push()`.
    pub fn set_descriptors(&mut self, descriptors: &[u8]) {
        debug_assert!(self.starts.is_empty());
        self.program_descriptors.extend_from_slice(descriptors);
    }

    /// Adds an elementary stream to the current section.
    ///
    /// - `stream_type` — type of program element
    /// - `pid` — TS Packet Identifier for this elementary stream
    /// - `descriptors` — raw ES-level descriptor bytes
    pub fn push(&mut self, stream_type: u8, pid: u16, descriptors: &[u8]) {
        debug_assert!(!self.finalized);
        debug_assert!(pid < PID_NONE);

        let item_size = PMT_ITEM_HEADER_SIZE + descriptors.len();

        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let last_start = *self.starts.last().unwrap();
            let current_size = self.buffer.len() - last_start;
            if current_size + item_size + PMT_CRC_SIZE > PMT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.push(stream_type);
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => pid,
        ));
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 4 => 0b1111,
            es_info_length: 12 => descriptors.len() as u16,
        ));
        self.buffer.extend_from_slice(descriptors);
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    /// Returns a [`Sections`] collection referencing the internal buffer.
    pub fn finalize(&mut self) -> Sections<'_> {
        debug_assert!(!self.finalized);
        self.finalized = true;

        if self.starts.is_empty() {
            self.begin_section();
        }

        self.seal_section();

        let last_section_number = (self.starts.len() - 1) as u8;

        for i in 0 .. self.starts.len() {
            let start = self.starts[i];
            let end = if i + 1 < self.starts.len() {
                self.starts[i + 1]
            } else {
                self.buffer.len()
            };

            // Patch section_length: total section bytes - 3
            let section_length = (end - start - 3) as u16;
            self.buffer[start + 1 .. start + 3].copy_from_slice(&pack_bits!(u16,
                section_syntax_indicator: 1 => 1,
                private_bit: 1 => 0,
                reserved1: 2 => 0b11,
                section_length: 12 => section_length,
            ));

            // Patch section_number and last_section_number
            self.buffer[start + 6] = i as u8;
            self.buffer[start + 7] = last_section_number;

            // Compute and write CRC32
            let crc = crc32b(&self.buffer[start .. end - PMT_CRC_SIZE]);
            self.buffer[end - PMT_CRC_SIZE .. end].copy_from_slice(&crc.to_be_bytes());
        }

        Sections::new(&self.buffer, &self.starts)
    }

    /// Writes the 12-byte section header and program descriptors.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());

        // Bytes 0-7: table_id, section_length, pnr, version, section/last_section
        self.buffer.extend_from_slice(&pack_bits!(u64,
            table_id: 8 => PMT_TABLE_ID,
            section_syntax_indicator: 1 => 1,
            private_bit: 1 => 0,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            program_number: 16 => self.pnr,
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
        self.buffer.extend_from_slice(&self.program_descriptors);
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; PMT_CRC_SIZE]);
    }
}
