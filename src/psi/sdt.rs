use crate::bytes::*;
use crate::psi::{Psi, PsiDemux, Descriptors};

pub const SDT_PID: u16 = 0x0011;

/// Maximum section length, exclude PSI header and CRC
const SDT_MAX_SIZE: usize = 1024 - 3 - 4;

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
        let mut item = Self::default();

        item.pnr = slice[0 ..].get_u16();
        item.eit_schedule_flag = (slice[2] >> 1) & 0x01;
        item.eit_present_following_flag = slice[2] & 0x01;
        item.running_status = (slice[3] >> 5) & 0x07;
        item.free_ca_mode = (slice[3] >> 4) & 0x01;

        item.descriptors.parse(&slice[5 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 5, 0x00);

        buffer[skip ..].set_u16(self.pnr);
        buffer[skip + 2] = 0xFC | (self.eit_schedule_flag << 1) | self.eit_present_following_flag;

        let flags_3 = (self.running_status << 5) | (self.free_ca_mode << 4);
        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        buffer[skip + 3 ..].set_u16((u16::from(flags_3) << 8) | descriptors_len);
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
    pub items: Vec<SdtItem>
}

impl Sdt {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 11 + 4 &&
        (psi.buffer[0] & 0xFB) == 0x42 && /* 0x42 or 0x46 */
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.tsid = psi.buffer[3 ..].get_u16();
        self.version = (psi.buffer[5] & 0x3E) >> 1;
        self.onid = psi.buffer[8 ..].get_u16();

        let ptr = &psi.buffer[11 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 5 {
            let item_len = 5 + (ptr[skip + 3 ..].get_u16() & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(SdtItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self) -> Psi {
        let mut psi = Psi::new(self.table_id, 11, self.version);
        psi.buffer[1] = 0xF0;  // set section_syntax_indicator and reserved bits
        psi.buffer[3 ..].set_u16(self.tsid);
        psi.buffer[8 ..].set_u16(self.onid);
        psi.buffer[10] = 0xFF;  // reserved_future_use
        psi
    }
}

impl PsiDemux for Sdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init()];

        for item in &self.items {
            {
                let psi = psi_list.last_mut().unwrap();
                if SDT_MAX_SIZE >= psi.buffer.len() + item.size() {
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
