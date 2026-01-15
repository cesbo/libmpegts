use crate::psi::{
    Descriptors,
    Psi,
    PsiDemux,
};

pub const SDT_PID: u16 = 0x0011;

/// Maximum section length without CRC
const SDT_SECTION_SIZE: usize = 1024 - 4;

/// SDT item.
#[derive(Debug, Default)]
pub struct SdtItem {
    /// Program number.
    pub pnr: u16,
    /// Indicates that EIT schedule information for the service is present in the current TS.
    pub eit_schedule_flag: u8,
    /// Indicates that EIT_present_following information for the service is present in the current TS.
    pub eit_present_following_flag: u8,
    /// Indicating the status of the service.
    pub running_status: u8,
    /// Indicates that all the component streams of the service are not scrambled.
    pub free_ca_mode: u8,
    /// List of descriptors.
    pub descriptors: Descriptors,
}

impl SdtItem {
    fn parse(slice: &[u8]) -> Self {
        let mut item = Self {
            pnr: u16::from_be_bytes([slice[0], slice[1]]),
            eit_schedule_flag: (slice[2] >> 1) & 0x01,
            eit_present_following_flag: slice[2] & 0x01,
            running_status: (slice[3] >> 5) & 0x07,
            free_ca_mode: (slice[3] >> 4) & 0x01,
            ..Default::default()
        };

        item.descriptors.parse(&slice[5 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.pnr.to_be_bytes());
        buffer.push(0xFC | (self.eit_schedule_flag << 1) | self.eit_present_following_flag);

        let skip = buffer.len();
        buffer.extend_from_slice(&[0x00, 0x00]); // placeholder for flags and descriptors length
        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        let flags_3 = (self.running_status << 5) | (self.free_ca_mode << 4);
        let flags_and_len = (u16::from(flags_3) << 8) | descriptors_len;
        buffer[skip .. skip + 2].copy_from_slice(&flags_and_len.to_be_bytes());
    }

    #[inline]
    fn size(&self) -> usize {
        5 + self.descriptors.size()
    }
}

/// Service Description Table - contains data describing the services
/// in the system e.g. names of services, the service provider, etc.
///
/// EN 300 468 - 5.2.3
#[derive(Debug, Default)]
pub struct Sdt {
    /// Identifies to which table the section belongs:
    /// * `0x42` - actual TS
    /// * `0x46` - other TS
    pub table_id: u8,
    /// SDT version.
    pub version: u8,
    /// Transport stream identifier.
    pub tsid: u16,
    /// Identifying the network of the originating delivery system.
    pub onid: u16,
    /// List of SDT items.
    pub items: Vec<SdtItem>,
}

impl Sdt {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 11 + 4 &&
        (psi.buffer[0] & 0xFB) == 0x42 && /* 0x42 or 0x46 */
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.tsid = u16::from_be_bytes([psi.buffer[3], psi.buffer[4]]);
        self.version = (psi.buffer[5] & 0x3E) >> 1;
        self.onid = u16::from_be_bytes([psi.buffer[8], psi.buffer[9]]);

        let ptr = &psi.buffer[11 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 5 {
            let item_len =
                5 + (u16::from_be_bytes([ptr[skip + 3], ptr[skip + 4]]) & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items
                .push(SdtItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self) -> Psi {
        let mut psi = Psi::new(self.table_id, 3, self.version);
        psi.buffer[1] = 0xF0; // set section_syntax_indicator and reserved bits
        psi.buffer.extend_from_slice(&self.tsid.to_be_bytes());
        psi.buffer.push(0xC0 | ((self.version << 1) & 0x3E) | 0x01);
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder for section_number and last_section_number
        psi.buffer.extend_from_slice(&self.onid.to_be_bytes());
        psi.buffer.push(0xFF); // reserved_future_use
        psi
    }
}

impl PsiDemux for Sdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init()];

        for item in &self.items {
            {
                let psi = psi_list.last_mut().unwrap();
                if SDT_SECTION_SIZE >= psi.buffer.len() + item.size() {
                    item.assemble(&mut psi.buffer);
                    continue;
                }
            }

            let mut psi = self.psi_init();
            item.assemble(&mut psi.buffer);
            psi_list.push(psi);
        }

        psi_list
    }
}

impl From<&Psi> for Sdt {
    fn from(psi: &Psi) -> Self {
        let mut sdt = Sdt::default();
        sdt.parse(psi);
        sdt
    }
}
