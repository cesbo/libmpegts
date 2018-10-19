use base;
use psi::Psi;
use psi::descriptors::Descriptors;

pub const SDT_PID: u16 = 0x11;

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
        item.eit_schedule_flag = slice[2] & 0x01;
        item.running_status = (slice[3] >> 5) & 0x07;
        item.free_ca_mode = (slice[3] >> 4) & 0x01;

        item.descriptors.parse(&slice[5 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 5, 0x00);

        {
            let ptr = buffer.as_mut_slice();
            base::set_u16(&mut ptr[skip ..], self.pnr);
            ptr[skip + 2] = 0xFC | (self.eit_schedule_flag << 1) | self.eit_present_following_flag;
            ptr[skip + 3] = (self.running_status << 5) | (self.free_ca_mode << 4);
        }

        self.descriptors.assemble(buffer);

        let descs_len = buffer.len() - skip - 5;
        if descs_len > 0 {
            let ptr = buffer.as_mut_slice();
            base::set_u12(&mut ptr[skip + 3 ..], descs_len as u16);
        }
    }
}


/// Service Description Table - contains data describing the services
/// in the system e.g. names of services, the service provider, etc.
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

    pub fn assemble(&self, psi: &mut Psi) {
        psi.init(self.table_id);
        psi.buffer[1] = 0xF0;  // set section_syntax_indicator and reserved bits
        psi.buffer.resize(11, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.tsid);
        base::set_u16(&mut psi.buffer[8 ..], self.onid);
        psi.buffer[10] = 0xFF;  // reserved_future_use

        for item in &self.items {
            item.assemble(&mut psi.buffer);
        }

        psi.finalize();
    }
}