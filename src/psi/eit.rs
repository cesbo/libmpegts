use base;
use psi::{Psi, PsiDemux, PsiDemuxItem, Descriptors};

pub const EIT_PID: u16 = 0x12;
const EIT_MAX_SIZE: usize = 4096;

/// EIT Item
#[derive(Debug, Default)]
pub struct EitItem {
    /// Event identification number
    pub event_id: u16,
    /// Event start time in UTC
    pub start: i64,
    /// Event duration in seconds
    pub duration: i32,
    /// Indicating the status of the event
    /// * `0` - undefined
    /// * `1` - not running
    /// * `2` - starts in a few seconds (e.g. for video recording)
    /// * `3` - pausing
    /// * `4` - running
    /// * `5` - service off-air
    pub status: u8,
    /// indicates that access is controlled by a CA system
    pub ca_mode: u8,
    /// list of descriptors
    pub descriptors: Descriptors,
}

impl EitItem {
    fn parse(slice: &[u8]) -> Self {
        let mut item = EitItem::default();

        item.event_id = base::get_u16(&slice[0 ..]);
        item.start = base::get_mjd_date(&slice[2 ..]) +
            i64::from(base::get_bcd_time(&slice[4 ..]));
        item.duration = base::get_bcd_time(&slice[7 ..]);
        item.status = (slice[10] >> 5) & 0x07;
        item.ca_mode = (slice[10] >> 4) & 0x01;

        item.descriptors.parse(&slice[12 ..]);

        item
    }
}

impl PsiDemuxItem for EitItem {
    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 12, 0x00);

        base::set_u16(&mut buffer[skip ..], self.event_id);
        base::set_mjd_date(&mut buffer[skip + 2 ..], self.start);
        base::set_bcd_time(&mut buffer[skip + 4 ..], self.start as i32);
        base::set_bcd_time(&mut buffer[skip + 7 ..], self.duration);
        buffer[skip + 10] = ((self.status & 0x07) << 5) | ((self.ca_mode & 0x01) << 0x04);

        self.descriptors.assemble(buffer);

        let descs_len = buffer.len() - skip - 12;
        if descs_len > 0 {
            base::set_u12(&mut buffer[skip + 10 ..], descs_len as u16);
        }
    }

    #[inline]
    fn size(&self) -> usize {
        8 + self.descriptors.size()
    }
}

/// Event Information Table provides information in chronological order
/// regarding the events contained within each service.
#[derive(Debug, Default)]
pub struct Eit {
    /// identifies to which table the section belongs:
    /// * `0x4E` - actual TS, present/following event information
    /// * `0x4F` - other TS, present/following event information
    /// * `0x50 ... 0x5F` - actual TS, event schedule information
    /// * `0x60 ... 0x6F` - other TS, event schedule information
    pub table_id: u8,
    /// EIT version
    pub version: u8,
    /// program number
    pub pnr: u16,
    /// transport stream identifier
    pub tsid: u16,
    /// identifying the network of the originating delivery system
    pub onid: u16,
    /// list of EIT items
    pub items: Vec<EitItem>,
}

impl Eit {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 14 + 4 &&
        match psi.buffer[0] {
            0x4E => true,           /* actual TS, present/following */
            0x4F => true,           /* other TS, present/following */
            0x50 ... 0x5F => true,   /* actual TS, schedule */
            0x60 ... 0x6F => true,   /* other TS, schedule */
            _ => false,
        } &&
        psi.check()

        // TODO: check if PSI already parsed
    }

    /// Reads [`Psi`] and append data into the `Eit`
    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.version = psi.get_version();
        self.pnr = base::get_u16(&psi.buffer[3 ..]);
        self.tsid = base::get_u16(&psi.buffer[8 ..]);
        self.onid = base::get_u16(&psi.buffer[10 ..]);

        let ptr = &psi.buffer[14 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 12 {
            let item_len = 12 + base::get_u12(&ptr[skip + 10 ..]) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(EitItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self) -> Psi {
        let mut psi = Psi::default();
        psi.init(self.table_id);
        psi.buffer[1] = 0xF0; // set reserved_future_use bit
        psi.buffer.resize(14, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.pnr);
        base::set_u16(&mut psi.buffer[8 ..], self.tsid);
        base::set_u16(&mut psi.buffer[10 ..], self.onid);
        // WTF: psi.buffer[12] - segment_last_section_number
        psi.buffer[13] = self.table_id;
        psi
    }
}

impl PsiDemux for Eit {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init()];

        for item in &self.items {
            {
                let mut psi = psi_list.last_mut().unwrap();
                if EIT_MAX_SIZE >= psi.buffer.len() + item.size() {
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
