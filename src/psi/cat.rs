use crate::{
    pack_bits,
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        Sections,
        check_crc32,
        finalize_sections,
        psi_section_length,
    },
};

/// TS Packet Identifier for CAT
pub const CAT_PID: u16 = 0x0001;
const CAT_TABLE_ID: u8 = 0x01;
const CAT_HEADER_SIZE: usize = 8;
const CAT_CRC_SIZE: usize = 4;
const CAT_SECTION_SIZE: usize = 1024;

/// Conditional Access Table associates EMM streams with CA systems: the
/// section body is a loop of CA_descriptors.
pub struct CatSectionRef<'a>(&'a [u8]);

impl<'a> CatSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// CAT version
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// List of descriptors.
    pub fn descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let end = self.0.len() - CAT_CRC_SIZE;
        (end > CAT_HEADER_SIZE).then(|| self.0[CAT_HEADER_SIZE .. end].into())
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - CAT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for CatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < CAT_HEADER_SIZE + CAT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != CAT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(CatSectionRef(&value[.. section_length]))
    }
}

impl<'a> TryFrom<&'a Psi> for CatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => CatSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}

/// CAT section generation config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatConfig {
    pub version: u8,
    /// Raw descriptor bytes for the section body
    pub descriptors: Vec<u8>,
}

/// One-shot CAT (Conditional Access Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{CatBuilder, CatConfig, CatSectionRef, Desc09, Descriptor};
///
/// let mut descriptors = Vec::new();
/// Desc09 {
///     ca_system_id: 0x0963,
///     ca_pid: 1200,
///     private_data: &[],
/// }
/// .encode(&mut descriptors)
/// .unwrap();
///
/// let sections = CatBuilder::build(CatConfig {
///     version: 0,
///     descriptors,
/// });
/// assert_eq!(sections.len(), 1);
/// let cat = CatSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(cat.version(), 0);
/// ```
pub struct CatBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    version: u8,
}

impl CatBuilder {
    /// Converts a CAT config into finalized PSI sections. Descriptor bytes are
    /// split across sections at descriptor boundaries; a truncated trailing
    /// descriptor is passed through as-is.
    pub fn build(config: CatConfig) -> Sections {
        let mut builder = Self {
            buffer: Vec::with_capacity(CAT_SECTION_SIZE),
            starts: Vec::new(),
            version: config.version & 0x1f,
        };

        let data = &config.descriptors;
        let mut offset = 0;
        while offset < data.len() {
            let mut end = data.len();
            if offset + 2 <= data.len() {
                end = end.min(offset + 2 + data[offset + 1] as usize);
            }
            builder.push(&data[offset .. end]);
            offset = end;
        }

        builder.finalize()
    }

    /// Adds one descriptor to the current section.
    fn push(&mut self, descriptor: &[u8]) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let last_section_start = *self.starts.last().unwrap();
            let current_section_size = self.buffer.len() - last_section_start;
            if current_section_size + descriptor.len() + CAT_CRC_SIZE > CAT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.extend_from_slice(descriptor);
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
            table_id: 8 => CAT_TABLE_ID,
            section_syntax_indicator: 1 => 1,
            private_bit: 1 => 0,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            reserved2: 18 => 0x3ffff,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; CAT_CRC_SIZE]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::{
        Desc09,
        Desc09Ref,
        Descriptor,
    };

    #[test]
    fn builds_empty_cat() {
        let sections = CatBuilder::build(CatConfig {
            version: 3,
            descriptors: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let cat = CatSectionRef::try_from(&sections[0][..]).unwrap();
        assert_eq!(cat.table_id(), 0x01);
        assert_eq!(cat.version(), 3);
        assert!(cat.descriptors().is_none());
    }

    #[test]
    fn builds_cat_with_ca_descriptor() {
        let mut descriptors = Vec::new();
        Desc09 {
            ca_system_id: 0x0963,
            ca_pid: 0x04b0,
            private_data: &[],
        }
        .encode(&mut descriptors)
        .unwrap();

        let sections = CatBuilder::build(CatConfig {
            version: 0,
            descriptors,
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            &section[.. CAT_HEADER_SIZE],
            [0x01, 0xb0, 0x0f, 0xff, 0xff, 0xc1, 0x00, 0x00]
        );

        let cat = CatSectionRef::try_from(section).unwrap();
        let desc = cat
            .descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let ca = Desc09Ref::try_from(desc).unwrap();
        assert_eq!(ca.ca_system_id(), 0x0963);
        assert_eq!(ca.ca_pid(), 0x04b0);
    }

    #[test]
    fn splits_oversized_descriptor_loop() {
        // 10 descriptors of 257 bytes each exceed one 1024-byte section
        let mut descriptors = Vec::new();
        for i in 0 .. 10 {
            Desc09 {
                ca_system_id: 0x0963,
                ca_pid: 0x0100 + i,
                private_data: &[0; 251],
            }
            .encode(&mut descriptors)
            .unwrap();
        }

        let sections = CatBuilder::build(CatConfig {
            version: 0,
            descriptors,
        });

        assert!(sections.len() > 1);
        let last_section_number = (sections.len() - 1) as u8;

        let mut count = 0;
        for i in 0 .. sections.len() {
            let section = &sections[i];
            assert!(section.len() <= CAT_SECTION_SIZE);
            assert_eq!(section[6], i as u8);
            assert_eq!(section[7], last_section_number);

            let cat = CatSectionRef::try_from(section).unwrap();
            for desc in cat.descriptors().unwrap() {
                let ca = Desc09Ref::try_from(desc.unwrap()).unwrap();
                assert_eq!(ca.ca_pid(), 0x0100 + count);
                count += 1;
            }
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn passes_truncated_descriptor_through() {
        // Declared length 16, only 1 byte present
        let sections = CatBuilder::build(CatConfig {
            version: 0,
            descriptors: vec![0x09, 0x10, 0x01],
        });

        assert_eq!(sections.len(), 1);
        let cat = CatSectionRef::try_from(&sections[0][..]).unwrap();
        let mut iter = cat.descriptors().unwrap().into_iter();
        assert!(iter.next().unwrap().is_err());
    }
}
