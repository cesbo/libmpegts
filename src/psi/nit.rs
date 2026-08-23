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

pub const NIT_PID: u16 = 0x0010;

/// Table ID of NIT describing the actual network
pub const NIT_TABLE_ID_ACTUAL: u8 = 0x40;
/// Table ID of NIT describing another network
pub const NIT_TABLE_ID_OTHER: u8 = 0x41;

const NIT_HEADER_SIZE: usize = 10;
const NIT_TS_LOOP_LENGTH_SIZE: usize = 2;
const NIT_ITEM_HEADER_SIZE: usize = 6;
const NIT_CRC_SIZE: usize = 4;
const NIT_SECTION_SIZE: usize = 1024;

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
        (self.0.len() > NIT_ITEM_HEADER_SIZE).then(|| self.0[NIT_ITEM_HEADER_SIZE ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for NitTransportStreamRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < NIT_ITEM_HEADER_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[4], value[5]]) & 0x0fff) as usize;
        let item_length = NIT_ITEM_HEADER_SIZE + desc_length;
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
        (descriptors_len > 0)
            .then(|| self.0[NIT_HEADER_SIZE .. NIT_HEADER_SIZE + descriptors_len].into())
    }

    /// Iterator for NIT transport streams
    pub fn transport_streams(&self) -> NitTransportStreamIter<'a> {
        let items_start = NIT_HEADER_SIZE + self.descriptors_length() + NIT_TS_LOOP_LENGTH_SIZE;
        let items_end = self.0.len() - NIT_CRC_SIZE;
        NitTransportStreamIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - NIT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for NitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < NIT_HEADER_SIZE + NIT_TS_LOOP_LENGTH_SIZE + NIT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            NIT_TABLE_ID_ACTUAL | NIT_TABLE_ID_OTHER => (),
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

/// Transport stream entry for [`NitConfig`].
#[derive(Clone)]
pub struct NitStream {
    pub transport_stream_id: u16,
    pub original_network_id: u16,
    /// Raw descriptor bytes for the transport stream loop
    pub descriptors: Vec<u8>,
}

/// NIT section generation config.
pub struct NitConfig {
    /// [`NIT_TABLE_ID_ACTUAL`] or [`NIT_TABLE_ID_OTHER`]
    pub table_id: u8,
    pub network_id: u16,
    pub version: u8,
    /// Raw descriptor bytes for the network descriptor loop
    pub network_descriptors: Vec<u8>,
    pub streams: Vec<NitStream>,
}

/// One-shot NIT (Network Information Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{
///     Desc40, Desc41, Desc41Item, Descriptor, NIT_TABLE_ID_ACTUAL, NitBuilder, NitConfig,
///     NitSectionRef, NitStream,
/// };
/// use libmpegts::utils::textcode::Charset;
///
/// let mut network_descriptors = Vec::new();
/// Desc40 {
///     name: "Network",
///     charset: Charset::Iso6937,
/// }
/// .encode(&mut network_descriptors)
/// .unwrap();
///
/// let mut descriptors = Vec::new();
/// Desc41 {
///     items: &[Desc41Item {
///         service_id: 1,
///         service_type: 1,
///     }],
/// }
/// .encode(&mut descriptors)
/// .unwrap();
///
/// let sections = NitBuilder::build(NitConfig {
///     table_id: NIT_TABLE_ID_ACTUAL,
///     network_id: 1,
///     version: 0,
///     network_descriptors,
///     streams: vec![NitStream {
///         transport_stream_id: 1,
///         original_network_id: 85,
///         descriptors,
///     }],
/// });
/// assert_eq!(sections.len(), 1);
/// let nit = NitSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(nit.network_id(), 1);
/// ```
pub struct NitBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    /// Position of transport_stream_loop_length in the current section
    ts_loop_pos: Option<usize>,
    table_id: u8,
    network_id: u16,
    version: u8,
}

impl NitBuilder {
    /// Converts a NIT config into finalized PSI sections. Network descriptor
    /// bytes are split across sections at descriptor boundaries; a truncated
    /// trailing descriptor is passed through as-is.
    pub fn build(config: NitConfig) -> Sections {
        debug_assert!(matches!(
            config.table_id,
            NIT_TABLE_ID_ACTUAL | NIT_TABLE_ID_OTHER
        ));

        let mut builder = Self {
            buffer: Vec::with_capacity(NIT_SECTION_SIZE),
            starts: Vec::new(),
            ts_loop_pos: None,
            table_id: config.table_id,
            network_id: config.network_id,
            version: config.version & 0x1f,
        };

        let data = &config.network_descriptors;
        let mut offset = 0;
        while offset < data.len() {
            let mut end = data.len();
            if offset + 2 <= data.len() {
                end = end.min(offset + 2 + data[offset + 1] as usize);
            }
            builder.push_network_descriptor(&data[offset .. end]);
            offset = end;
        }

        for stream in config.streams {
            builder.push_stream(stream);
        }

        builder.finalize()
    }

    /// Adds one network descriptor to the current section.
    fn push_network_descriptor(&mut self, descriptor: &[u8]) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let current_section_size = self.buffer.len() - self.starts.last().unwrap();
            let tail_size = descriptor.len() + NIT_TS_LOOP_LENGTH_SIZE + NIT_CRC_SIZE;
            if current_section_size + tail_size > NIT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.extend_from_slice(descriptor);
    }

    /// Adds a transport stream to the current section.
    fn push_stream(&mut self, stream: NitStream) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let current_section_size = self.buffer.len() - self.starts.last().unwrap();
            let mut item_size = NIT_ITEM_HEADER_SIZE + stream.descriptors.len();
            if self.ts_loop_pos.is_none() {
                item_size += NIT_TS_LOOP_LENGTH_SIZE;
            }
            if current_section_size + item_size + NIT_CRC_SIZE > NIT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }
        self.open_ts_loop();

        self.buffer
            .extend_from_slice(&stream.transport_stream_id.to_be_bytes());
        self.buffer
            .extend_from_slice(&stream.original_network_id.to_be_bytes());
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved_future_use: 4 => 0b1111,
            transport_descriptors_length: 12 => stream.descriptors.len() as u16,
        ));
        self.buffer.extend_from_slice(&stream.descriptors);
    }

    /// Finalizes all sections: patches headers, computes CRC32.
    fn finalize(mut self) -> Sections {
        if self.starts.is_empty() {
            self.begin_section();
        }

        self.seal_section();

        finalize_sections(self.buffer, self.starts)
    }

    /// Writes the 10-byte section header template and registers a new section start.
    fn begin_section(&mut self) {
        self.starts.push(self.buffer.len());
        self.ts_loop_pos = None;
        self.buffer.extend_from_slice(&pack_bits!(u64,
            table_id: 8 => self.table_id,
            section_syntax_indicator: 1 => 1,
            reserved_future_use: 1 => 1,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            network_id: 16 => self.network_id,
            reserved2: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved_future_use: 4 => 0b1111,
            network_descriptors_length: 12 => 0, // placeholder, patched in open_ts_loop()
        ));
    }

    /// Patches network_descriptors_length and opens the transport stream loop
    /// of the current section.
    fn open_ts_loop(&mut self) {
        if self.ts_loop_pos.is_some() {
            return;
        }

        let start = *self.starts.last().unwrap();
        let length = self.buffer.len() - start - NIT_HEADER_SIZE;
        self.buffer[start + 8] = 0xf0 | ((length >> 8) as u8 & 0x0f);
        self.buffer[start + 9] = length as u8;

        self.ts_loop_pos = Some(self.buffer.len());
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved_future_use: 4 => 0b1111,
            transport_stream_loop_length: 12 => 0, // placeholder, patched in seal_section()
        ));
    }

    /// Patches transport_stream_loop_length and appends CRC32 placeholder
    /// bytes to seal the current section.
    fn seal_section(&mut self) {
        self.open_ts_loop();

        let pos = self.ts_loop_pos.take().unwrap();
        let length = self.buffer.len() - pos - NIT_TS_LOOP_LENGTH_SIZE;
        self.buffer[pos] = 0xf0 | ((length >> 8) as u8 & 0x0f);
        self.buffer[pos + 1] = length as u8;

        self.buffer.extend_from_slice(&[0x00; NIT_CRC_SIZE]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        psi::{
            Desc40,
            Desc40Ref,
            Desc41,
            Desc41Item,
            Desc41Ref,
            Descriptor,
        },
        utils::textcode::Charset,
    };

    fn stream(transport_stream_id: u16, descriptors: Vec<u8>) -> NitStream {
        NitStream {
            transport_stream_id,
            original_network_id: 85,
            descriptors,
        }
    }

    #[test]
    fn builds_empty_nit() {
        let sections = NitBuilder::build(NitConfig {
            table_id: NIT_TABLE_ID_ACTUAL,
            network_id: 1,
            version: 3,
            network_descriptors: Vec::new(),
            streams: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            &section[.. NIT_HEADER_SIZE + NIT_TS_LOOP_LENGTH_SIZE],
            [
                0x40, 0xf0, 0x0d, 0x00, 0x01, 0xc7, 0x00, 0x00, 0xf0, 0x00, 0xf0, 0x00
            ]
        );

        let nit = NitSectionRef::try_from(section).unwrap();
        assert_eq!(nit.table_id(), 0x40);
        assert_eq!(nit.version(), 3);
        assert_eq!(nit.network_id(), 1);
        assert!(nit.network_descriptors().is_none());
        assert_eq!(nit.transport_streams().count(), 0);
    }

    #[test]
    fn builds_nit_with_descriptors() {
        let mut network_descriptors = Vec::new();
        Desc40 {
            name: "Astra Net",
            charset: Charset::Iso6937,
        }
        .encode(&mut network_descriptors)
        .unwrap();

        let mut descriptors = Vec::new();
        Desc41 {
            items: &[
                Desc41Item {
                    service_id: 1,
                    service_type: 1,
                },
                Desc41Item {
                    service_id: 2,
                    service_type: 2,
                },
            ],
        }
        .encode(&mut descriptors)
        .unwrap();

        let sections = NitBuilder::build(NitConfig {
            table_id: NIT_TABLE_ID_OTHER,
            network_id: 0x2000,
            version: 0,
            network_descriptors,
            streams: vec![stream(100, descriptors)],
        });

        assert_eq!(sections.len(), 1);
        let nit = NitSectionRef::try_from(&sections[0][..]).unwrap();
        assert_eq!(nit.table_id(), 0x41);
        assert_eq!(nit.network_id(), 0x2000);

        let desc = nit
            .network_descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let name = Desc40Ref::try_from(desc).unwrap();
        assert_eq!(name.name_text().unwrap().to_string(), "Astra Net");

        let ts = nit.transport_streams().next().unwrap().unwrap();
        assert_eq!(ts.transport_stream_id(), 100);
        assert_eq!(ts.original_network_id(), 85);
        let desc = ts
            .transport_stream_descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let list = Desc41Ref::try_from(desc).unwrap();
        let items: Vec<Desc41Item> = list.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].service_id, 2);
        assert_eq!(items[1].service_type, 2);
    }

    #[test]
    fn splits_oversized_stream_loop() {
        // 10 streams with 251-byte descriptor loops exceed one 1024-byte section
        let streams: Vec<NitStream> = (0 .. 10)
            .map(|i| {
                let mut descriptors = vec![0xf2, 249];
                descriptors.extend_from_slice(&[0; 249]);
                stream(i, descriptors)
            })
            .collect();

        let sections = NitBuilder::build(NitConfig {
            table_id: NIT_TABLE_ID_ACTUAL,
            network_id: 1,
            version: 0,
            network_descriptors: Vec::new(),
            streams,
        });

        assert!(sections.len() > 1);
        let last_section_number = (sections.len() - 1) as u8;

        let mut count = 0;
        for i in 0 .. sections.len() {
            let section = &sections[i];
            assert!(section.len() <= NIT_SECTION_SIZE);
            assert_eq!(section[6], i as u8);
            assert_eq!(section[7], last_section_number);

            let nit = NitSectionRef::try_from(section).unwrap();
            assert!(nit.network_descriptors().is_none());
            for ts in nit.transport_streams() {
                assert_eq!(ts.unwrap().transport_stream_id(), count);
                count += 1;
            }
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn splits_oversized_network_descriptors() {
        // 10 descriptors of 257 bytes each exceed one 1024-byte section
        let mut network_descriptors = Vec::new();
        for i in 0 .. 10 {
            network_descriptors.extend_from_slice(&[0xf2, 255, i]);
            network_descriptors.extend_from_slice(&[0; 254]);
        }

        let sections = NitBuilder::build(NitConfig {
            table_id: NIT_TABLE_ID_ACTUAL,
            network_id: 1,
            version: 0,
            network_descriptors,
            streams: vec![stream(7, Vec::new())],
        });

        assert!(sections.len() > 1);

        let mut count = 0;
        let mut stream_count = 0;
        for i in 0 .. sections.len() {
            let nit = NitSectionRef::try_from(&sections[i][..]).unwrap();
            if let Some(descriptors) = nit.network_descriptors() {
                for desc in descriptors {
                    assert_eq!(desc.unwrap().data()[0], count);
                    count += 1;
                }
            }
            for ts in nit.transport_streams() {
                assert_eq!(ts.unwrap().transport_stream_id(), 7);
                stream_count += 1;
            }
        }
        assert_eq!(count, 10);
        assert_eq!(stream_count, 1);
    }

    #[test]
    fn passes_truncated_network_descriptor_through() {
        // Declared length 16, only 1 byte present
        let sections = NitBuilder::build(NitConfig {
            table_id: NIT_TABLE_ID_ACTUAL,
            network_id: 1,
            version: 0,
            network_descriptors: vec![0xf2, 0x10, 0x01],
            streams: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let nit = NitSectionRef::try_from(&sections[0][..]).unwrap();
        let mut iter = nit.network_descriptors().unwrap().into_iter();
        assert!(iter.next().unwrap().is_err());
    }
}
