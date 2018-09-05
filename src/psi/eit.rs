use base::*;
use psi::Psi;
use psi::descriptors::*;

pub const EIT_PID: u16 = 0x12;

/// EIT Item
///
/// # Fields
///
/// * `event_id` - identification number of the described event
/// * `start` - start time in UTC
/// * `duration` - duration of the event in seconds
/// * `status` - indicating the status of the event
///     * `0` - undefined
///     * `1` - not running
///     * `2` - starts in a few seconds (e.g. for video recording)
///     * `3` - pausing
///     * `4` - running
///     * `5` - service off-air
/// * `ca_mode` - indicates that access is controlled by a CA system
/// * `descriptors` - list of descriptors
#[derive(Debug, Default)]
pub struct EitItem {
    pub event_id: u16,
    pub start: i64,
    pub duration: i32,
    pub status: u8,
    pub ca_mode: u8,
    pub descriptors: Descriptors,
}

impl EitItem {
    fn parse(slice: &[u8]) -> Self {
        let mut item = EitItem::default();

        item.event_id = get_u16(&slice[0 ..]);
        item.start = get_mjd_date(&slice[2 ..]) + get_bcd_time(&slice[4 ..]) as i64;
        item.duration = get_bcd_time(&slice[7 ..]);
        item.status = (slice[10] >> 5) & 0x07;
        item.ca_mode = (slice[10] >> 4) & 0x01;

        item.descriptors.parse(&slice[12 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 12, 0x00);

        {
            let ptr = buffer.as_mut_slice();
            set_u16(&mut ptr[skip + 0 ..], self.event_id);
            set_mjd_date(&mut ptr[skip + 2 ..], self.start);
            set_bcd_time(&mut ptr[skip + 4 ..], self.start as i32);
            set_bcd_time(&mut ptr[skip + 7 ..], self.duration);
            ptr[skip + 10] = ((self.status & 0x07) << 5) | ((self.ca_mode & 0x01) << 0x04);
        }

        self.descriptors.assemble(buffer);

        let descs_len = buffer.len() - skip - 12;
        if descs_len > 0 {
            let ptr = buffer.as_mut_slice();
            set_u12(&mut ptr[skip + 10 ..], descs_len as u16);
        }
    }
}

/// Event Information Table provides information in chronological order
/// regarding the events contained within each service.
///
/// # Fields
///
/// * `table_id` - identifies to which table the section belongs:
///     * `0x4E` - actual TS, present/following event information
///     * `0x4F` - other TS, present/following event information
///     * `0x50 ... 0x5F` - actual TS, event schedule information
///     * `0x60 ... 0x6F` - other TS, event schedule information
/// * `version` - EIT version
/// * `pnr` - program number
/// * `tsid` - transport stream identifier
/// * `onid` - identifying the network of the originating delivery system
/// * `items` - list of EIT items
#[derive(Debug, Default)]
pub struct Eit {
    pub table_id: u8,
    pub version: u8,
    pub pnr: u16,
    pub tsid: u16,
    pub onid: u16,
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
        self.pnr = get_u16(&psi.buffer[3 ..]);
        self.tsid = get_u16(&psi.buffer[8 ..]);
        self.onid = get_u16(&psi.buffer[10 ..]);

        let ptr = &psi.buffer[14 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 12 {
            let item_len = 12 + get_u12(&ptr[skip + 10 ..]) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(EitItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    /// Converts `Eit` into [`Psi`]
    pub fn assemble(&self, psi: &mut Psi) {
        psi.init(self.table_id);
        psi.buffer[1] = 0xF0; // set reserved_future_use bit
        psi.buffer.resize(14, 0x00);
        psi.set_version(self.version);
        set_u16(&mut psi.buffer[3 ..], self.pnr);
        set_u16(&mut psi.buffer[8 ..], self.tsid);
        set_u16(&mut psi.buffer[10 ..], self.onid);
        // WTF: psi.buffer[12] - segment_last_section_number
        psi.buffer[13] = self.table_id;

        for item in self.items.iter() {
            item.assemble(&mut psi.buffer);
        }

        psi.finalize();
    }
}
