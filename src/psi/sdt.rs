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

pub const SDT_PID: u16 = 0x0011;

/// Table ID of SDT describing the actual transport stream
pub const SDT_TABLE_ID_ACTUAL: u8 = 0x42;
/// Table ID of SDT describing another transport stream
pub const SDT_TABLE_ID_OTHER: u8 = 0x46;

const SDT_HEADER_SIZE: usize = 11;
const SDT_ITEM_HEADER_SIZE: usize = 5;
const SDT_CRC_SIZE: usize = 4;
const SDT_SECTION_SIZE: usize = 1024;

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
        let items_start = SDT_HEADER_SIZE;
        let items_end = self.0.len() - SDT_CRC_SIZE;
        SdtServiceIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - SDT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for SdtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < SDT_HEADER_SIZE + SDT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            SDT_TABLE_ID_ACTUAL | SDT_TABLE_ID_OTHER => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(SdtSectionRef(&value[.. section_length]))
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

/// Service entry for [`SdtConfig`].
#[derive(Clone)]
pub struct SdtService {
    pub service_id: u16,
    pub eit_schedule_flag: bool,
    pub eit_present_following_flag: bool,
    pub running_status: u8,
    pub free_ca_mode: bool,
    /// Raw descriptor bytes for the service loop
    pub service_descriptors: Vec<u8>,
}

/// SDT section generation config.
pub struct SdtConfig {
    /// [`SDT_TABLE_ID_ACTUAL`] or [`SDT_TABLE_ID_OTHER`]
    pub table_id: u8,
    pub transport_stream_id: u16,
    pub original_network_id: u16,
    pub version: u8,
    pub services: Vec<SdtService>,
}

/// One-shot SDT (Service Description Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{
///     Desc48, Descriptor, SDT_TABLE_ID_ACTUAL, SdtBuilder, SdtConfig, SdtSectionRef, SdtService,
/// };
/// use libmpegts::utils::textcode::Charset;
///
/// let mut descriptors = Vec::new();
/// Desc48 {
///     service_type: 1,
///     provider_name: "Provider",
///     service_name: "Channel One",
///     charset: Charset::Iso6937,
/// }
/// .encode(&mut descriptors)
/// .unwrap();
///
/// let sections = SdtBuilder::build(SdtConfig {
///     table_id: SDT_TABLE_ID_ACTUAL,
///     transport_stream_id: 1,
///     original_network_id: 85,
///     version: 0,
///     services: vec![SdtService {
///         service_id: 1,
///         eit_schedule_flag: false,
///         eit_present_following_flag: true,
///         running_status: 4,
///         free_ca_mode: false,
///         service_descriptors: descriptors,
///     }],
/// });
/// assert_eq!(sections.len(), 1);
/// let sdt = SdtSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(sdt.transport_stream_id(), 1);
/// ```
pub struct SdtBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    table_id: u8,
    transport_stream_id: u16,
    original_network_id: u16,
    version: u8,
}

impl SdtBuilder {
    /// Converts an SDT config into finalized PSI sections.
    pub fn build(config: SdtConfig) -> Sections {
        debug_assert!(matches!(
            config.table_id,
            SDT_TABLE_ID_ACTUAL | SDT_TABLE_ID_OTHER
        ));

        let mut builder = Self {
            buffer: Vec::with_capacity(SDT_SECTION_SIZE),
            starts: Vec::new(),
            table_id: config.table_id,
            transport_stream_id: config.transport_stream_id,
            original_network_id: config.original_network_id,
            version: config.version & 0x1f,
        };

        for service in config.services {
            builder.push(service);
        }

        builder.finalize()
    }

    /// Adds a service to the current section.
    fn push(&mut self, service: SdtService) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let last_section_start = *self.starts.last().unwrap();
            let current_section_size = self.buffer.len() - last_section_start;
            let item_size = SDT_ITEM_HEADER_SIZE + service.service_descriptors.len();
            if current_section_size + item_size + SDT_CRC_SIZE > SDT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer
            .extend_from_slice(&service.service_id.to_be_bytes());
        self.buffer.extend_from_slice(&pack_bits!(u8,
            reserved: 6 => 0b111111,
            eit_schedule_flag: 1 => service.eit_schedule_flag,
            eit_present_following_flag: 1 => service.eit_present_following_flag,
        ));
        self.buffer.extend_from_slice(&pack_bits!(u16,
            running_status: 3 => service.running_status,
            free_ca_mode: 1 => service.free_ca_mode,
            descriptors_loop_length: 12 => service.service_descriptors.len() as u16,
        ));
        self.buffer.extend_from_slice(&service.service_descriptors);
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    fn finalize(mut self) -> Sections {
        if self.starts.is_empty() {
            self.begin_section();
        }

        self.seal_section();

        finalize_sections(self.buffer, self.starts)
    }

    /// Writes the 11-byte section header template and registers a new section start.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());
        self.buffer.extend_from_slice(&pack_bits!(u64,
            table_id: 8 => self.table_id,
            section_syntax_indicator: 1 => 1,
            reserved_future_use: 1 => 1,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            transport_stream_id: 16 => self.transport_stream_id,
            reserved2: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));
        self.buffer
            .extend_from_slice(&self.original_network_id.to_be_bytes());
        self.buffer.push(0xff); // reserved_future_use
    }

    /// Appends CRC32 placeholder bytes to seal the current section.
    fn seal_section(&mut self) {
        self.buffer.extend_from_slice(&[0x00; SDT_CRC_SIZE]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        psi::{
            Descriptor,
            Desc48,
        },
        utils::textcode::Charset,
    };

    fn service(service_id: u16, service_descriptors: Vec<u8>) -> SdtService {
        SdtService {
            service_id,
            eit_schedule_flag: false,
            eit_present_following_flag: true,
            running_status: 4,
            free_ca_mode: false,
            service_descriptors,
        }
    }

    #[test]
    fn builds_empty_sdt() {
        let sections = SdtBuilder::build(SdtConfig {
            table_id: SDT_TABLE_ID_ACTUAL,
            transport_stream_id: 1,
            original_network_id: 85,
            version: 7,
            services: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let sdt = SdtSectionRef::try_from(&sections[0][..]).unwrap();
        assert_eq!(sdt.table_id(), 0x42);
        assert_eq!(sdt.version(), 7);
        assert_eq!(sdt.transport_stream_id(), 1);
        assert_eq!(sdt.original_network_id(), 85);
        assert_eq!(sdt.services().count(), 0);
    }

    #[test]
    fn builds_sdt_with_service() {
        let sections = SdtBuilder::build(SdtConfig {
            table_id: SDT_TABLE_ID_ACTUAL,
            transport_stream_id: 1,
            original_network_id: 0x0055,
            version: 2,
            services: vec![SdtService {
                service_id: 0x0001,
                eit_schedule_flag: false,
                eit_present_following_flag: true,
                running_status: 4,
                free_ca_mode: true,
                service_descriptors: Vec::new(),
            }],
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            &section[.. SDT_HEADER_SIZE + SDT_ITEM_HEADER_SIZE],
            [
                0x42, 0xf0, 0x11, 0x00, 0x01, 0xc5, 0x00, 0x00, 0x00, 0x55, 0xff, 0x00, 0x01,
                0xfd, 0x90, 0x00
            ]
        );

        let sdt = SdtSectionRef::try_from(&sections[0][..]).unwrap();
        let service = sdt.services().next().unwrap().unwrap();
        assert_eq!(service.service_id(), 1);
        assert!(!service.eit_schedule_flag());
        assert!(service.eit_present_following_flag());
        assert_eq!(service.running_status(), 4);
        assert!(service.free_ca_mode());
        assert!(service.service_descriptors().is_none());
    }

    #[test]
    fn builds_sdt_with_service_descriptor() {
        let mut descriptors = Vec::new();
        Desc48 {
            service_type: 1,
            provider_name: "Provider",
            service_name: "Channel One",
            charset: Charset::Iso6937,
        }
        .encode(&mut descriptors)
        .unwrap();

        let sections = SdtBuilder::build(SdtConfig {
            table_id: SDT_TABLE_ID_ACTUAL,
            transport_stream_id: 1,
            original_network_id: 85,
            version: 0,
            services: vec![service(1, descriptors)],
        });

        let sdt = SdtSectionRef::try_from(&sections[0][..]).unwrap();
        let entry = sdt.services().next().unwrap().unwrap();
        let desc = entry
            .service_descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let service = crate::psi::Desc48Ref::try_from(desc).unwrap();
        assert_eq!(service.provider_name_text().unwrap().to_string(), "Provider");
        assert_eq!(service.service_name_text().unwrap().to_string(), "Channel One");
    }

    #[test]
    fn splits_oversized_service_loop() {
        // 10 services with 252-byte descriptor loops exceed one 1024-byte section
        let services: Vec<SdtService> = (0 .. 10)
            .map(|i| {
                let mut descriptors = vec![0xf2, 250];
                descriptors.extend_from_slice(&[0; 250]);
                service(i, descriptors)
            })
            .collect();

        let sections = SdtBuilder::build(SdtConfig {
            table_id: SDT_TABLE_ID_OTHER,
            transport_stream_id: 1,
            original_network_id: 85,
            version: 0,
            services,
        });

        assert!(sections.len() > 1);
        let last_section_number = (sections.len() - 1) as u8;

        let mut count = 0;
        for i in 0 .. sections.len() {
            let section = &sections[i];
            assert!(section.len() <= SDT_SECTION_SIZE);
            assert_eq!(section[6], i as u8);
            assert_eq!(section[7], last_section_number);

            let sdt = SdtSectionRef::try_from(section).unwrap();
            assert_eq!(sdt.table_id(), 0x46);
            for entry in sdt.services() {
                assert_eq!(entry.unwrap().service_id(), count);
                count += 1;
            }
        }
        assert_eq!(count, 10);
    }
}
