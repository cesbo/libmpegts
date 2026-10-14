use crate::{
    pack_bits,
    psi::{
        DescriptorsRef,
        PsiSectionError,
        Sections,
        check_crc32,
        finalize_sections,
        psi_section_length,
    },
};

/// Table ID of BAT
pub const BAT_TABLE_ID: u8 = 0x4a;

const BAT_HEADER_SIZE: usize = 10;
const BAT_TS_LOOP_LENGTH_SIZE: usize = 2;
const BAT_ITEM_HEADER_SIZE: usize = 6;
const BAT_CRC_SIZE: usize = 4;
const BAT_SECTION_SIZE: usize = 1024;

/// Transport stream entry of the BAT transport stream loop.
pub struct BatTransportStreamRef<'a>(&'a [u8]);

impl<'a> BatTransportStreamRef<'a> {
    /// Transport Stream Identifier
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// Original Network ID
    pub fn original_network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[2], self.0[3]])
    }

    /// Transport stream descriptors
    pub fn transport_stream_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > BAT_ITEM_HEADER_SIZE).then(|| self.0[BAT_ITEM_HEADER_SIZE ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for BatTransportStreamRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < BAT_ITEM_HEADER_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[4], value[5]]) & 0x0fff) as usize;
        let item_length = BAT_ITEM_HEADER_SIZE + desc_length;
        if value.len() >= item_length {
            Ok(BatTransportStreamRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

/// Iterator over the BAT transport stream loop.
pub struct BatTransportStreamIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for BatTransportStreamIter<'a> {
    type Item = Result<BatTransportStreamRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match BatTransportStreamRef::try_from(remaining) {
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

/// The BAT provides information regarding bouquets: the bouquet name
/// and a list of services for each bouquet. Carried on
/// [`SDT_PID`](super::SDT_PID) beside the SDT.
///
/// EN 300 468 - 5.2.2
pub struct BatSectionRef<'a>(&'a [u8]);

impl<'a> BatSectionRef<'a> {
    /// Table ID, always [`BAT_TABLE_ID`]
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// BAT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// `true` when the section carries the currently applicable information,
    /// `false` when it describes the next (future) version.
    pub fn current_next_indicator(&self) -> bool {
        (self.0[5] & 0x01) != 0
    }

    /// Number of this section within the sub-table.
    pub fn section_number(&self) -> u8 {
        self.0[6]
    }

    /// Number of the last section of this sub-table.
    pub fn last_section_number(&self) -> u8 {
        self.0[7]
    }

    /// Bouquet ID
    pub fn bouquet_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[8], self.0[9]]) & 0x0fff) as usize
    }

    /// List of bouquet descriptors.
    pub fn bouquet_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        (descriptors_len > 0)
            .then(|| self.0[BAT_HEADER_SIZE .. BAT_HEADER_SIZE + descriptors_len].into())
    }

    /// Iterator for BAT transport streams
    pub fn transport_streams(&self) -> BatTransportStreamIter<'a> {
        let items_start = BAT_HEADER_SIZE + self.descriptors_length() + BAT_TS_LOOP_LENGTH_SIZE;
        let items_end = self.0.len() - BAT_CRC_SIZE;
        BatTransportStreamIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - BAT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for BatSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        const MIN_SIZE: usize = BAT_HEADER_SIZE + BAT_TS_LOOP_LENGTH_SIZE + BAT_CRC_SIZE;

        if value.len() < MIN_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != BAT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length < MIN_SIZE || section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        // bouquet descriptors and the transport_stream_loop_length field
        // must fit before the CRC, or the getters would slice out of bounds
        let section = BatSectionRef(&value[.. section_length]);
        if MIN_SIZE + section.descriptors_length() > section_length {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        Ok(section)
    }
}

/// Transport stream entry for [`BatConfig`].
#[derive(Clone)]
pub struct BatStream {
    pub transport_stream_id: u16,
    pub original_network_id: u16,
    /// Raw descriptor bytes for the transport stream loop
    pub descriptors: Vec<u8>,
}

/// BAT section generation config.
pub struct BatConfig {
    pub bouquet_id: u16,
    pub version: u8,
    /// Raw descriptor bytes for the bouquet descriptor loop
    pub bouquet_descriptors: Vec<u8>,
    pub streams: Vec<BatStream>,
}

/// One-shot BAT (Bouquet Association Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{
///     BatBuilder, BatConfig, BatSectionRef, BatStream, Desc41, Desc41Item, Descriptor,
/// };
///
/// // bouquet_name_descriptor (0x47) as raw bytes
/// let bouquet_descriptors = vec![0x47, 3, b'B', b'A', b'T'];
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
/// let sections = BatBuilder::build(BatConfig {
///     bouquet_id: 0x1001,
///     version: 0,
///     bouquet_descriptors,
///     streams: vec![BatStream {
///         transport_stream_id: 1,
///         original_network_id: 85,
///         descriptors,
///     }],
/// });
/// assert_eq!(sections.len(), 1);
/// let bat = BatSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(bat.bouquet_id(), 0x1001);
/// ```
pub struct BatBuilder {
    buffer: Vec<u8>,
    starts: Vec<usize>,
    /// Position of transport_stream_loop_length in the current section
    ts_loop_pos: Option<usize>,
    bouquet_id: u16,
    version: u8,
}

impl BatBuilder {
    /// Converts a BAT config into finalized PSI sections. Bouquet descriptor
    /// bytes are split across sections at descriptor boundaries; a truncated
    /// trailing descriptor is passed through as-is.
    pub fn build(config: BatConfig) -> Sections {
        let mut builder = Self {
            buffer: Vec::with_capacity(BAT_SECTION_SIZE),
            starts: Vec::new(),
            ts_loop_pos: None,
            bouquet_id: config.bouquet_id,
            version: config.version & 0x1f,
        };

        let data = &config.bouquet_descriptors;
        let mut offset = 0;
        while offset < data.len() {
            let mut end = data.len();
            if offset + 2 <= data.len() {
                end = end.min(offset + 2 + data[offset + 1] as usize);
            }
            builder.push_bouquet_descriptor(&data[offset .. end]);
            offset = end;
        }

        for stream in config.streams {
            builder.push_stream(stream);
        }

        builder.finalize()
    }

    /// Adds one bouquet descriptor to the current section.
    fn push_bouquet_descriptor(&mut self, descriptor: &[u8]) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let current_section_size = self.buffer.len() - self.starts.last().unwrap();
            let tail_size = descriptor.len() + BAT_TS_LOOP_LENGTH_SIZE + BAT_CRC_SIZE;
            if current_section_size + tail_size > BAT_SECTION_SIZE {
                self.seal_section();
                self.begin_section();
            }
        }

        self.buffer.extend_from_slice(descriptor);
    }

    /// Adds a transport stream to the current section.
    fn push_stream(&mut self, stream: BatStream) {
        if self.starts.is_empty() {
            self.begin_section();
        } else {
            let current_section_size = self.buffer.len() - self.starts.last().unwrap();
            let mut item_size = BAT_ITEM_HEADER_SIZE + stream.descriptors.len();
            if self.ts_loop_pos.is_none() {
                item_size += BAT_TS_LOOP_LENGTH_SIZE;
            }
            if current_section_size + item_size + BAT_CRC_SIZE > BAT_SECTION_SIZE {
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
            table_id: 8 => BAT_TABLE_ID,
            section_syntax_indicator: 1 => 1,
            reserved_future_use: 1 => 1,
            reserved1: 2 => 0b11,
            section_length: 12 => 0, // placeholder, patched in finalize()
            bouquet_id: 16 => self.bouquet_id,
            reserved2: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1,
            section_number: 8 => 0, // placeholder, patched in finalize()
            last_section_number: 8 => 0, // placeholder, patched in finalize()
        ));
        self.buffer.extend_from_slice(&pack_bits!(u16,
            reserved_future_use: 4 => 0b1111,
            bouquet_descriptors_length: 12 => 0, // placeholder, patched in open_ts_loop()
        ));
    }

    /// Patches bouquet_descriptors_length and opens the transport stream loop
    /// of the current section.
    fn open_ts_loop(&mut self) {
        if self.ts_loop_pos.is_some() {
            return;
        }

        let start = *self.starts.last().unwrap();
        let length = self.buffer.len() - start - BAT_HEADER_SIZE;
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
        let length = self.buffer.len() - pos - BAT_TS_LOOP_LENGTH_SIZE;
        self.buffer[pos] = 0xf0 | ((length >> 8) as u8 & 0x0f);
        self.buffer[pos + 1] = length as u8;

        self.buffer.extend_from_slice(&[0x00; BAT_CRC_SIZE]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::{
        Desc41,
        Desc41Item,
        Desc41Ref,
        Descriptor,
        PsiSectionMut,
    };

    fn stream(transport_stream_id: u16, descriptors: Vec<u8>) -> BatStream {
        BatStream {
            transport_stream_id,
            original_network_id: 85,
            descriptors,
        }
    }

    fn build_one(descriptors: Vec<u8>) -> Vec<u8> {
        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 1,
            version: 0,
            bouquet_descriptors: Vec::new(),
            streams: vec![stream(100, descriptors)],
        });
        sections[0].to_vec()
    }

    /// Applies `patch` to the section bytes and recomputes the CRC32.
    fn patched(mut section: Vec<u8>, patch: impl FnOnce(&mut [u8])) -> Vec<u8> {
        let mut psi = PsiSectionMut::try_from(&mut section[..]).unwrap();
        patch(psi.as_mut());
        psi.update_crc32();
        section
    }

    #[test]
    fn builds_empty_bat() {
        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 0x1001,
            version: 3,
            bouquet_descriptors: Vec::new(),
            streams: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            &section[.. BAT_HEADER_SIZE + BAT_TS_LOOP_LENGTH_SIZE],
            [
                0x4a, 0xf0, 0x0d, 0x10, 0x01, 0xc7, 0x00, 0x00, 0xf0, 0x00, 0xf0, 0x00
            ]
        );

        let bat = BatSectionRef::try_from(section).unwrap();
        assert_eq!(bat.table_id(), BAT_TABLE_ID);
        assert_eq!(bat.version(), 3);
        assert!(bat.current_next_indicator());
        assert_eq!(bat.section_number(), 0);
        assert_eq!(bat.last_section_number(), 0);
        assert_eq!(bat.bouquet_id(), 0x1001);
        assert!(bat.bouquet_descriptors().is_none());
        assert_eq!(bat.transport_streams().count(), 0);
        assert_eq!(bat.crc32().to_be_bytes(), section[section.len() - 4 ..]);
    }

    #[test]
    fn builds_bat_with_descriptors() {
        // bouquet_name_descriptor (EN 300 468 6.2.4) as raw bytes
        let bouquet_descriptors = vec![0x47, 3, b'B', b'A', b'T'];

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

        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 0x2000,
            version: 0,
            bouquet_descriptors,
            streams: vec![stream(100, descriptors)],
        });

        assert_eq!(sections.len(), 1);
        let bat = BatSectionRef::try_from(&sections[0][..]).unwrap();
        assert_eq!(bat.bouquet_id(), 0x2000);

        let desc = bat
            .bouquet_descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(desc.tag(), 0x47);
        assert_eq!(desc.data(), b"BAT");

        let ts = bat.transport_streams().next().unwrap().unwrap();
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
        let streams: Vec<BatStream> = (0 .. 10)
            .map(|i| stream(i, [0xf2, 249].into_iter().chain([0; 249]).collect()))
            .collect();

        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 1,
            version: 0,
            bouquet_descriptors: Vec::new(),
            streams,
        });

        assert!(sections.len() > 1);
        let last_section_number = (sections.len() - 1) as u8;

        let mut count = 0;
        for (i, section) in sections.iter().enumerate() {
            assert!(section.len() <= BAT_SECTION_SIZE);
            let bat = BatSectionRef::try_from(section).unwrap();
            assert_eq!(bat.section_number(), i as u8);
            assert_eq!(bat.last_section_number(), last_section_number);
            assert!(bat.bouquet_descriptors().is_none());
            for ts in bat.transport_streams() {
                assert_eq!(ts.unwrap().transport_stream_id(), count);
                count += 1;
            }
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn splits_oversized_bouquet_descriptors() {
        // 10 descriptors of 257 bytes each exceed one 1024-byte section
        let mut bouquet_descriptors = Vec::new();
        for i in 0 .. 10 {
            bouquet_descriptors.extend_from_slice(&[0xf2, 255, i]);
            bouquet_descriptors.extend_from_slice(&[0; 254]);
        }

        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 1,
            version: 0,
            bouquet_descriptors,
            streams: vec![stream(7, Vec::new())],
        });

        assert!(sections.len() > 1);

        let mut count = 0;
        let mut stream_count = 0;
        for section in sections.iter() {
            assert!(section.len() <= BAT_SECTION_SIZE);
            let bat = BatSectionRef::try_from(section).unwrap();
            if let Some(descriptors) = bat.bouquet_descriptors() {
                for desc in descriptors {
                    assert_eq!(desc.unwrap().data()[0], count);
                    count += 1;
                }
            }
            for ts in bat.transport_streams() {
                assert_eq!(ts.unwrap().transport_stream_id(), 7);
                stream_count += 1;
            }
        }
        assert_eq!(count, 10);
        assert_eq!(stream_count, 1);
    }

    #[test]
    fn passes_truncated_bouquet_descriptor_through() {
        // Declared length 16, only 1 byte present
        let sections = BatBuilder::build(BatConfig {
            bouquet_id: 1,
            version: 0,
            bouquet_descriptors: vec![0xf2, 0x10, 0x01],
            streams: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let bat = BatSectionRef::try_from(&sections[0][..]).unwrap();
        let mut iter = bat.bouquet_descriptors().unwrap().into_iter();
        assert!(iter.next().unwrap().is_err());
    }

    #[test]
    fn rejects_truncated_section() {
        let mut section = build_one(Vec::new());
        // cut short of section_length, then of the fixed fields
        for len in [section.len() - 1, 15] {
            assert!(matches!(
                BatSectionRef::try_from(&section[.. len]),
                Err(PsiSectionError::InvalidSectionLength)
            ));
        }

        // section_length too small for the fixed fields
        section[2] = 12;
        assert!(matches!(
            BatSectionRef::try_from(&section[..]),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn rejects_wrong_table_id() {
        let section = patched(build_one(Vec::new()), |s| s[0] = 0x42);
        assert!(matches!(
            BatSectionRef::try_from(&section[..]),
            Err(PsiSectionError::InvalidTableId)
        ));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut section = build_one(Vec::new());
        let last = section.len() - 1;
        section[last] ^= 0x01;
        assert!(matches!(
            BatSectionRef::try_from(&section[..]),
            Err(PsiSectionError::InvalidCrc32)
        ));
    }

    #[test]
    fn rejects_oversized_descriptors_length() {
        // bouquet_descriptors_length runs past the CRC
        let section = patched(build_one(Vec::new()), |s| s[9] = 0xff);
        assert!(matches!(
            BatSectionRef::try_from(&section[..]),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn stops_on_truncated_stream() {
        // transport_descriptors_length is the low 12 bits of item bytes 4..6,
        // the item starts after the 10-byte header and 2-byte loop length
        let section = patched(build_one(vec![0xf2, 0]), |s| s[17] = 0x10);
        let bat = BatSectionRef::try_from(&section[..]).unwrap();
        let mut iter = bat.transport_streams();
        assert!(matches!(
            iter.next(),
            Some(Err(PsiSectionError::InvalidSectionLength))
        ));
        assert!(iter.next().is_none());
    }
}
