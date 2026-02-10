/// Program Association Table (PAT) implementation
use crate::{
    pack_bits,
    psi::{
        Psi,
        PsiSectionError,
        Sections,
        psi_section_length,
    },
    ts::PID_NONE,
    utils::crc32b,
};

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;
const PAT_TABLE_ID: u8 = 0x00;
const PAT_HEADER_SIZE: usize = 8;
const PAT_ITEM_SIZE: usize = 4;
const PAT_CRC_SIZE: usize = 4;
const PAT_SECTION_SIZE: usize = 1024;

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
        if value.len() < PAT_ITEM_SIZE {
            Err(PsiSectionError::InvalidSectionLength)
        } else {
            Ok(PatItemRef(&value[0 .. PAT_ITEM_SIZE]))
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
        let ptr = &self.0[PAT_HEADER_SIZE .. self.0.len() - PAT_CRC_SIZE];
        ptr.chunks(PAT_ITEM_SIZE).map(PatItemRef::try_from)
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - PAT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for PatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < PAT_HEADER_SIZE + PAT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != PAT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        let pat = PatSectionRef(&value[.. section_length]);

        let checksum = crc32b(&value[.. section_length - PAT_CRC_SIZE]);
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

/// Builder for PAT (Program Association Table) sections.
///
/// # Examples
///
/// ```
/// use mpegts::psi::{PatBuilder, PatSectionRef};
///
/// let mut builder = PatBuilder::new(1);
/// builder.push(0, 16);
/// builder.push(1, 100);
/// let sections = builder.finalize();
/// assert_eq!(sections.len(), 1);
/// let pat = PatSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(pat.tsid(), 1);
/// ```
pub struct PatBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    tsid: u16,
    version: u8,
}

impl PatBuilder {
    /// Creates a new PAT builder and begins the first section.
    ///
    /// - `tsid` — Transport Stream ID
    pub fn new(tsid: u16) -> Self {
        Self {
            buffer: Vec::with_capacity(PAT_SECTION_SIZE),
            starts: Vec::new(),
            tsid,
            version: 0,
        }
    }

    /// Sets PAT version (0..31).
    pub fn set_version(&mut self, version: u8) {
        self.version = version & 0x1f;
    }

    /// Adds a program mapping to the current section.
    ///
    /// - `pnr` — Program Number (0 = NIT PID)
    /// - `pid` — PMT PID (or NIT PID when pnr is 0)
    pub fn push(&mut self, pnr: u16, pid: u16) {
        debug_assert!(pid < PID_NONE);

        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let last_section_start = *self.starts.last().unwrap();
            let current_section_size = self.buffer.len() - last_section_start;
            if current_section_size + PAT_ITEM_SIZE + PAT_CRC_SIZE > PAT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.extend_from_slice(&pnr.to_be_bytes());
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => pid,
        ));
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    /// Consumes the builder and returns a [`Sections`] collection.
    pub fn finalize(mut self) -> Sections {
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
            let crc = crc32b(&self.buffer[start .. end - PAT_CRC_SIZE]);
            self.buffer[end - PAT_CRC_SIZE .. end].copy_from_slice(&crc.to_be_bytes());
        }

        Sections::new(self.buffer, self.starts)
    }

    /// Writes the 8-byte section header template and registers a new section start.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());
        self.buffer.extend_from_slice(&pack_bits!(u64,
            table_id: 8 => PAT_TABLE_ID,
            section_syntax_indicator: 1 => 1,
            private_bit: 1 => 0,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            transport_stream_id: 16 => self.tsid,
            reserved2: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; PAT_CRC_SIZE]);
    }
}
