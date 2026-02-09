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

// PAT section constraints
const PAT_TABLE_ID: u8 = 0x00;
const PAT_CRC_SIZE: usize = 4;
const PAT_ITEM_SIZE: usize = 4;
const PAT_SECTION_SIZE: usize = 1024;

/// Builder for PAT (Program Association Table) sections.
///
/// # Examples
///
/// ```
/// use mpegts::psi::{PatBuilder, PatSectionRef};
///
/// let mut builder = PatBuilder::new(1, 1);
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
    finalized: bool,
}

impl PatBuilder {
    /// Creates a new PAT builder and begins the first section.
    ///
    /// - `tsid` — Transport Stream ID
    /// - `version` — table version (0..31)
    pub fn new(tsid: u16, version: u8) -> Self {
        debug_assert!(version < 32);

        let mut builder = Self {
            buffer: Vec::with_capacity(PAT_SECTION_SIZE),
            starts: Vec::new(),
            tsid,
            version,
            finalized: false,
        };
        builder.begin_section();
        builder
    }

    /// Adds a program mapping to the current section.
    ///
    /// - `pnr` — Program Number (0 = NIT PID)
    /// - `pid` — PMT PID (or NIT PID when pnr is 0)
    pub fn push(&mut self, pnr: u16, pid: u16) {
        debug_assert!(!self.finalized);
        debug_assert!(pid < PID_NONE);

        let last_start = *self.starts.last().unwrap();
        let current_size = self.buffer.len() - last_start;
        if current_size + PAT_ITEM_SIZE + PAT_CRC_SIZE > PAT_SECTION_SIZE {
            self.seal_section();
            self.begin_section();
        }

        self.buffer.extend_from_slice(&pnr.to_be_bytes());
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => pid,
        ));
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    /// Returns a [`Sections`] collection referencing the internal buffer.
    pub fn finalize(&mut self) -> Sections<'_> {
        debug_assert!(!self.finalized);
        self.finalized = true;

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
            self.buffer[start + 1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
            self.buffer[start + 2] = section_length as u8;

            // Patch section_number and last_section_number
            self.buffer[start + 6] = i as u8;
            self.buffer[start + 7] = last_section_number;

            // Compute and write CRC32
            let crc = crc32b(&self.buffer[start .. end - PAT_CRC_SIZE]);
            self.buffer[end - 4] = (crc >> 24) as u8;
            self.buffer[end - 3] = (crc >> 16) as u8;
            self.buffer[end - 2] = (crc >> 8) as u8;
            self.buffer[end - 1] = crc as u8;
        }

        Sections::new(&self.buffer, &self.starts)
    }

    /// Writes the 8-byte section header template and registers a new section start.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());
        self.buffer.extend_from_slice(&[
            PAT_TABLE_ID,
            0xb0,                              // section_syntax_indicator
            0x00,                              // section_length placeholder
            (self.tsid >> 8) as u8,            //
            self.tsid as u8,                   // transport_stream_id
            0xc0 | (self.version << 1) | 0x01, // reserved + version + current_next
            0x00,                              // section_number placeholder
            0x00,                              // last_section_number placeholder
        ]);
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; PAT_CRC_SIZE]);
    }
}
