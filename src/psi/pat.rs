/// Program Association Table (PAT) implementation
use crate::{
    pack_bits,
    psi::{
        PsiSectionError,
        Sections,
        check_crc32,
        finalize_sections,
        psi_section_length,
    },
    ts::PID_NONE,
};

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;
const PAT_TABLE_ID: u8 = 0x00;
const PAT_HEADER_SIZE: usize = 8;
const PAT_ITEM_SIZE: usize = 4;
const PAT_CRC_SIZE: usize = 4;
const PAT_SECTION_SIZE: usize = 1024;

/// PAT program mapping config item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatProgram {
    pub program_number: u16,
    pub pid: u16,
}

/// PAT section generation config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatConfig {
    pub transport_stream_id: u16,
    pub version: u8,
    pub programs: Vec<PatProgram>,
}

pub struct PatProgramRef<'a>(&'a [u8]);

impl<'a> PatProgramRef<'a> {
    /// Program Number
    pub fn program_number(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// TS Packet Identifier
    pub fn pid(&self) -> u16 {
        u16::from_be_bytes([self.0[2], self.0[3]]) & 0x1fff
    }
}

impl<'a> TryFrom<&'a [u8]> for PatProgramRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < PAT_ITEM_SIZE {
            Err(PsiSectionError::InvalidSectionLength)
        } else {
            Ok(PatProgramRef(&value[0 .. PAT_ITEM_SIZE]))
        }
    }
}

pub struct PatProgramIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for PatProgramIter<'a> {
    type Item = Result<PatProgramRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match PatProgramRef::try_from(remaining) {
            Ok(program) => {
                self.offset += PAT_ITEM_SIZE;
                Some(Ok(program))
            }
            Err(e) => {
                self.offset = self.data.len();
                Some(Err(e))
            }
        }
    }
}

/// Program Association Table provides the correspondence between a `program_number` and
/// the `pid` value of the TS packets which carry the program definition.
pub struct PatSectionRef<'a>(&'a [u8]);

impl<'a> PatSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// Transport Stream ID to identify actual stream from any other multiplex within a network
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// PAT version
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// Iterator for PAT programs
    pub fn programs(&self) -> PatProgramIter<'a> {
        PatProgramIter {
            data: &self.0[PAT_HEADER_SIZE .. self.0.len() - PAT_CRC_SIZE],
            offset: 0,
        }
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

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(PatSectionRef(&value[.. section_length]))
    }
}

/// One-shot PAT (Program Association Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{PatBuilder, PatConfig, PatProgram, PatSectionRef};
///
/// let sections = PatBuilder::build(PatConfig {
///     transport_stream_id: 1,
///     version: 0,
///     programs: vec![
///         PatProgram {
///             program_number: 0,
///             pid: 16,
///         },
///         PatProgram {
///             program_number: 1,
///             pid: 100,
///         },
///     ],
/// });
/// assert_eq!(sections.len(), 1);
/// let pat = PatSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(pat.transport_stream_id(), 1);
/// ```
pub struct PatBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    transport_stream_id: u16,
    version: u8,
}

impl PatBuilder {
    /// Converts a PAT config into finalized PSI sections.
    pub fn build(config: PatConfig) -> Sections {
        let mut builder = Self {
            buffer: Vec::with_capacity(PAT_SECTION_SIZE),
            starts: Vec::new(),
            transport_stream_id: config.transport_stream_id,
            version: config.version & 0x1f,
        };

        for program in config.programs {
            builder.push(program);
        }

        builder.finalize()
    }

    /// Adds a program mapping to the current section.
    fn push(&mut self, program: PatProgram) {
        debug_assert!(program.pid < PID_NONE);

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

        self.buffer
            .extend_from_slice(&program.program_number.to_be_bytes());
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => program.pid,
        ));
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    fn finalize(mut self) -> Sections {
        if self.starts.is_empty() {
            self.begin_section();
        }

        self.seal_section();

        finalize_sections(self.buffer, self.starts)
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
            transport_stream_id: 16 => self.transport_stream_id,
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
