use crate::{
    es::StreamType,
    psi::{
        Descriptors,
        Psi,
        PsiDemux,
    },
};

/// Maximum section length without CRC
const PMT_SECTION_SIZE: usize = 1024 - 4;

/// PMT Item.
#[derive(Debug, Default)]
pub struct PmtItem {
    /// This field specifying the type of program element
    /// carried within the packets with the PID.
    pub stream_type: u8,
    /// This field specifying the PID of the Transport Stream packets
    /// which carry the associated program element.
    pub pid: u16,
    /// List of descriptors.
    pub descriptors: Descriptors,
}

impl PmtItem {
    pub fn parse(slice: &[u8]) -> Self {
        let mut item = Self {
            stream_type: slice[0],
            pid: u16::from_be_bytes([slice[1], slice[2]]) & 0x1FFF,
            ..Default::default()
        };

        item.descriptors.parse(&slice[5 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(self.stream_type);
        buffer.extend_from_slice(&(0xE000 | self.pid).to_be_bytes());

        let skip = buffer.len();
        buffer.extend_from_slice(&[0x00, 0x00]); // placeholder
        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        let flags_and_len = 0xF000 | descriptors_len;
        buffer[skip .. skip + 2].copy_from_slice(&flags_and_len.to_be_bytes());
    }

    #[inline]
    fn size(&self) -> usize {
        5 + self.descriptors.size()
    }

    pub fn get_stream_type(&self) -> StreamType {
        match self.stream_type {
            // Video
            0x01 => StreamType::VIDEO, // ISO/IEC 11172 Video
            0x02 => StreamType::VIDEO, // ISO/IEC 13818-2 Video
            0x10 => StreamType::VIDEO, // ISO/IEC 14496-2 Visual
            0x1B => StreamType::VIDEO, // ISO/IEC 14496-10 Video | H.264
            0x24 => StreamType::VIDEO, // ISO/IEC 23008-2 Video | H.265
            // Audio
            0x03 => StreamType::AUDIO, // ISO/IEC 11172 Audio
            0x04 => StreamType::AUDIO, // ISO/IEC 13818-3 Audio
            0x0F => StreamType::AUDIO, // ISO/IEC 13818-7 Audio (ADTS)
            0x11 => StreamType::AUDIO, // ISO/IEC 14496-3 Audio (LATM)
            // Private Data
            0x05 => {
                for desc in self.descriptors.iter() {
                    if desc.tag() == 0x6F {
                        // application_signalling_descriptor
                        return StreamType::AIT;
                    }
                }
                StreamType::DATA
            }
            0x06 => {
                for desc in self.descriptors.iter() {
                    match desc.tag() {
                        0x56 => return StreamType::TTX,   // teletext_descriptor
                        0x59 => return StreamType::SUB,   // subtitling_descriptor
                        0x6A => return StreamType::AUDIO, // AC-3_descriptor
                        0x7A => return StreamType::AUDIO, // enhanced_AC-3_descriptor
                        0x81 => return StreamType::AUDIO, // AC-3 Audio
                        _ => {}
                    }
                }
                StreamType::DATA
            }
            _ => StreamType::DATA,
        }
    }
}

/// Program Map Table - provides the mappings between program numbers
/// and the program elements that comprise them.
#[derive(Debug, Default)]
pub struct Pmt {
    /// PMT version.
    pub version: u8,
    /// Program number.
    pub pnr: u16,
    /// PCR (Program Clock Reference) pid.
    pub pcr: u16,
    /// List of descriptors.
    pub descriptors: Descriptors,
    /// List of PMT items.
    pub items: Vec<PmtItem>,
}

impl Pmt {
    #[inline]
    pub fn check(&self, psi: &Psi) -> bool {
        psi.size >= 12 + 4 && psi.buffer[0] == 0x02 && psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.pnr = u16::from_be_bytes([psi.buffer[3], psi.buffer[4]]);
        self.version = (psi.buffer[5] & 0x3E) >> 1;
        self.pcr = u16::from_be_bytes([psi.buffer[8], psi.buffer[9]]) & 0x1FFF;

        let descriptors_len =
            (u16::from_be_bytes([psi.buffer[10], psi.buffer[11]]) & 0x0FFF) as usize;
        self.descriptors
            .parse(&psi.buffer[12 .. 12 + descriptors_len]);

        let ptr = &psi.buffer[12 + descriptors_len .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 5 {
            let item_len =
                5 + (u16::from_be_bytes([ptr[skip + 3], ptr[skip + 4]]) & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items
                .push(PmtItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self, first: bool) -> Psi {
        let mut psi = Psi::new(0x02, 3, self.version);
        psi.buffer.extend_from_slice(&self.pnr.to_be_bytes());
        psi.buffer
            .push(set_bits!(8, 0b11, 2, self.version, 5, 1, 1));
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder
        psi.buffer
            .extend_from_slice(&(0xE000 | self.pcr).to_be_bytes());

        let skip = psi.buffer.len();
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder
        let desc_len = if first {
            self.descriptors.assemble(&mut psi.buffer) as u16
        } else {
            0
        };
        psi.buffer[skip .. skip + 2].copy_from_slice(&(0xF000 | desc_len).to_be_bytes());

        psi
    }
}

impl PsiDemux for Pmt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init(true)];

        for item in &self.items {
            {
                let psi = psi_list.last_mut().unwrap();
                if PMT_SECTION_SIZE >= psi.buffer.len() + item.size() {
                    item.assemble(&mut psi.buffer);
                    continue;
                }
            }

            let mut psi = self.psi_init(false);
            item.assemble(&mut psi.buffer);
            psi_list.push(psi);
        }

        psi_list
    }
}

impl From<&Psi> for Pmt {
    fn from(psi: &Psi) -> Self {
        let mut pmt = Pmt::default();
        pmt.parse(psi);
        pmt
    }
}
