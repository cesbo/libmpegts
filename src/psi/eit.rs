// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::{
    bytes::Bytes,
    psi::{
        BCDTime,
        MJDFrom,
        MJDTo,
        Psi,
        PsiDemux,
        Descriptors,
    },
};


pub const EIT_PID: u16 = 0x0012;


/// Maximum section length without CRC
const EIT_SECTION_SIZE: usize = 4096 - 4;


/// EIT Item
#[derive(Debug, Default, Clone)]
pub struct EitItem {
    /// Event identification number
    pub event_id: u16,
    /// Event start time in UTC
    pub start: u64,
    /// Event duration in seconds
    pub duration: u32,
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

        item.event_id = slice[0 ..].get_u16();
        item.start = slice[2 ..].get_u16().from_mjd() +
            u64::from(slice[4 ..].get_u24().from_bcd_time());
        item.duration = slice[7 ..].get_u24().from_bcd_time();
        item.status = (slice[10] >> 5) & 0x07;
        item.ca_mode = (slice[10] >> 4) & 0x01;

        item.descriptors.parse(&slice[12 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 12, 0x00);

        buffer[skip ..].set_u16(self.event_id);
        buffer[skip + 2 ..].set_u16(self.start.to_mjd());
        buffer[skip + 4 ..].set_u24((self.start as u32).to_bcd_time());
        buffer[skip + 7 ..].set_u24(self.duration.to_bcd_time());

        let flags_10 = set_bits!(8,
            self.status, 3,
            self.ca_mode, 1);
        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        buffer[skip + 10 ..].set_u16((u16::from(flags_10) << 8) | descriptors_len);
    }

    #[inline]
    fn size(&self) -> usize {
        12 + self.descriptors.size()
    }
}


/// Event Information Table provides information in chronological order
/// regarding the events contained within each service.
#[derive(Debug, Default)]
pub struct Eit {
    /// identifies to which table the section belongs:
    /// * `0x4E` - actual TS, present/following event information
    /// * `0x4F` - other TS, present/following event information
    /// * `0x50 ..= 0x5F` - actual TS, event schedule information
    /// * `0x60 ..= 0x6F` - other TS, event schedule information
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
            0x50 ..= 0x5F => true,   /* actual TS, schedule */
            0x60 ..= 0x6F => true,   /* other TS, schedule */
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
        self.pnr = psi.buffer[3 ..].get_u16();
        self.version = (psi.buffer[5] & 0x3E) >> 1;
        self.tsid = psi.buffer[8 ..].get_u16();
        self.onid = psi.buffer[10 ..].get_u16();

        let ptr = &psi.buffer[14 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 12 {
            let item_len = 12 + (ptr[skip + 10 ..].get_u16() & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(EitItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self) -> Psi {
        let mut psi = Psi::new(self.table_id, 14, self.version);
        psi.buffer[1] = 0xF0; // set reserved_future_use bit
        psi.buffer[3 ..].set_u16(self.pnr);
        psi.buffer[8 ..].set_u16(self.tsid);
        psi.buffer[10 ..].set_u16(self.onid);
        psi
    }
}


impl PsiDemux for Eit {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list: Vec<Psi> = Vec::new();

        if self.items.is_empty() {
            return psi_list;
        }

        if self.table_id == 0x4E || self.table_id == 0x4F {
            let last_section_number = (self.items.len() - 1) as u8;
            for (n, item) in self.items.iter().enumerate() {
                let mut psi = self.psi_init();

                // Section_number
                psi.buffer[6] = n as u8;
                // Last_Section_number
                psi.buffer[7] = last_section_number;
                // Segment_last_Section_number
                psi.buffer[12] = last_section_number;
                // Last_table_id
                psi.buffer[13] = self.table_id;

                item.assemble(&mut psi.buffer);

                psi.finalize();

                psi_list.push(psi);
            }

            return psi_list;
        }

        const DAY_DURATION: u64 = 24 * 60 * 60;
        const SEG_DURATION: u64 = 3 * 60 * 60;

        let table_id = self.table_id & 0xF0;

        // Midnight
        let first_item = self.items.first().unwrap();
        let mut midnight = first_item.start / DAY_DURATION * DAY_DURATION;

        // Last table id
        let last_table_id = {
            let last_item = self.items.last().unwrap();
            let service_duration = last_item.start - midnight;
            let service_segments = service_duration / SEG_DURATION;
            table_id + (service_segments / 32) as u8
        };
        let mut current_table_id = self.table_id & 0xF0;

        let mut current_section: u8 = 0;

        // Fill segments with emtpy sections
        {
            let mut empty_eit = Eit::default();
            empty_eit.table_id = table_id;
            empty_eit.version = self.version;
            empty_eit.pnr = self.pnr;
            empty_eit.tsid = self.tsid;
            empty_eit.onid = self.onid;
            let mut psi = empty_eit.psi_init();

            let current_segment = (first_item.start - midnight) / (3 * 60 * 60);
            for _ in 0 ..= current_segment {
                psi.buffer[0] = current_table_id;
                psi.buffer[6] = current_section;

                psi_list.push(psi.clone());
                midnight += SEG_DURATION;
                current_section += 8;
            }
        }

        let mut next_midnight = midnight + SEG_DURATION;

        {
            let mut psi = self.psi_init();
            psi.buffer[6] = current_section;
            psi_list.push(psi);
        }

        for item in &self.items {
            let psi = psi_list.last_mut().unwrap();

            if item.start >= next_midnight {
                midnight = next_midnight;
                next_midnight = midnight + SEG_DURATION;

                current_section = current_section / 8 * 8;
                if current_section == 248 {
                    current_section = 0;
                    current_table_id += 1;
                } else {
                    current_section += 8;
                }

                let mut psi = self.psi_init();
                psi.buffer[0] = current_table_id;
                psi.buffer[6] = current_section;
                item.assemble(&mut psi.buffer);
                psi_list.push(psi);
                continue;
            }

            if item.size() + psi.buffer.len() >= EIT_SECTION_SIZE {
                current_section = current_section + 1;

                let mut psi = self.psi_init();
                psi.buffer[0] = current_table_id;
                psi.buffer[6] = current_section;
                item.assemble(&mut psi.buffer);
                psi_list.push(psi);
                continue;
            }

            item.assemble(&mut psi.buffer);
        }

        // TODO: fix Segment_last_Section_number

        current_table_id = 0x00;
        let mut last_section_number = 0x00;

        // Now current_section is last_section_number
        for psi in psi_list.iter_mut().rev() {
            if psi.buffer[0] != current_table_id {
                current_table_id = psi.buffer[0];
                last_section_number = psi.buffer[6];
            }

            // Last_Section_number
            psi.buffer[7] = last_section_number;
            // Segment_last_Section_number
            psi.buffer[12] = psi.buffer[6];
            // Last_table_id
            psi.buffer[13] = last_table_id;

            psi.finalize();
        }

        psi_list
    }

    /// Converts PSI into TS packets
    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        if psi_list.is_empty() {
            return;
        }

        for psi in psi_list.iter_mut() {
            psi.pid = pid;
            psi.cc = *cc;
            psi.demux(dst);
            *cc = psi.cc;
        }
    }
}


impl From<&Psi> for Eit {
    fn from(psi: &Psi) -> Self {
        let mut eit = Eit::default();
        eit.parse(psi);
        eit
    }
}
