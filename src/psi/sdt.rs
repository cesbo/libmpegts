use base;
use psi::{Psi, PsiDemux, PsiDemuxItem, Descriptors};

pub const SDT_PID: u16 = 0x11;
const SDT_MAX_SIZE: usize = 1024;

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

        item.pnr = base::get_u16(&slice[0 ..]);
        item.eit_schedule_flag = (slice[2] >> 1) & 0x01;
        item.eit_present_following_flag = slice[2] & 0x01;
        item.running_status = (slice[3] >> 5) & 0x07;
        item.free_ca_mode = (slice[3] >> 4) & 0x01;

        item.descriptors.parse(&slice[5 ..]);

        item
    }
}

impl PsiDemuxItem for SdtItem {
    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 5, 0x00);

        base::set_u16(&mut buffer[skip ..], self.pnr);
        buffer[skip + 2] = 0xFC | (self.eit_schedule_flag << 1) | self.eit_present_following_flag;
        buffer[skip + 3] = (self.running_status << 5) | (self.free_ca_mode << 4);

        self.descriptors.assemble(buffer);

        let descs_len = buffer.len() - skip - 5;
        if descs_len > 0 {
            base::set_u12(&mut buffer[skip + 3 ..], descs_len as u16);
        }
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
        match psi.buffer[0] {
            0x42 => true,  /* actual TS */
            0x46 => true,  /* other TS */
            _ => false
        } &&
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.version = psi.get_version();
        self.tsid = base::get_u16(&psi.buffer[3 ..]);
        self.onid = base::get_u16(&psi.buffer[8 ..]);

        let ptr = &psi.buffer[11 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 5 {
            let item_len = 5 + base::get_u12(&ptr[skip + 3 ..]) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(SdtItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self) -> Psi {
        let mut psi = Psi::default();
        psi.init(self.table_id);
        psi.buffer[1] = 0xF0;  // set section_syntax_indicator and reserved bits
        psi.buffer.resize(11, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.tsid);
        base::set_u16(&mut psi.buffer[8 ..], self.onid);
        psi.buffer[10] = 0xFF;  // reserved_future_use
        psi
    }
}

impl PsiDemux for Sdt {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init()];

        for item in &self.items {
            {
                let mut psi = psi_list.last_mut().unwrap();
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
