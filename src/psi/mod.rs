mod descriptors;
mod eit;
mod error;
mod nit;
mod pat;
mod pmt;
mod sdt;
mod tdt;
mod tot;

pub use descriptors::*;
pub use eit::*;
pub use error::*;
pub use nit::*;
pub use pat::*;
pub use pmt::*;
pub use sdt::*;
pub use tdt::*;
pub use tot::*;

use crate::{
    ts::{
        self,
        TsPacketRef,
    },
    utils::crc32b,
};

/// Program Specific Information includes normative data which is necessary for
/// the demultiplexing of transport streams and the successful regeneration of
/// programs.
#[derive(Debug, Clone)]
pub struct Psi {
    /// Buffer for assembling PSI section.
    /// Extra 184 bytes for safety to contain TS stuffing bytes
    data: [u8; 4096 + 184],
    data_length: usize,
    head: [u8; 184],
    head_length: usize,
    section_length: usize,
    assembling: bool,

    pub buffer: Vec<u8>,
    pub size: usize, // PSI size

    pub pid: u16,
    pub cc: u8,
}

impl Default for Psi {
    fn default() -> Psi {
        Psi {
            data: [0; 4096 + 184],
            data_length: 0,
            head: [0; 184],
            head_length: 0,
            section_length: 0,
            assembling: false,

            buffer: Vec::with_capacity(4095 + 184),
            size: 0,
            pid: 0,
            cc: 0,
        }
    }
}

impl PartialEq for Psi {
    fn eq(&self, other: &Psi) -> bool {
        self.size == other.size && self.get_crc32() == other.get_crc32()
    }
}

impl Psi {
    /// Init PSI packet
    ///
    /// - `table_id` - table identifier
    /// - `size` - header length
    /// - `version` - table version
    pub fn new(table_id: u8) -> Self {
        let mut psi = Psi::default();
        psi.buffer.extend_from_slice(&[table_id, 0xB0, 0x00]);
        psi
    }

    /// Clears the PSI buffer and all fields
    pub fn clear(&mut self) {
        self.data_length = 0;
        self.head_length = 0;
        self.section_length = 0;
        self.assembling = false;

        self.buffer.clear();
        self.size = 0;
    }

    /// Appends data to current section being assembled.
    /// Returns `true` if section_length is known (header received).
    fn append_data(&mut self, cc: u8, payload: &[u8]) -> bool {
        if !self.assembling {
            return false;
        }

        if cc != (self.cc + 1) & 0x0F {
            // Continuity counter error
            self.clear();
            return false;
        }

        if self.head_length > 0 {
            // Restore saved head from previous packet
            self.data[.. self.head_length].copy_from_slice(&self.head[.. self.head_length]);
            self.data_length = self.head_length;
            self.section_length = 0;
            self.head_length = 0;
        } else if self.data_length + payload.len() > self.data.len() {
            // Overflow
            self.clear();
            return false;
        }

        let end = self.data_length + payload.len();
        self.data[self.data_length .. end].copy_from_slice(payload);
        self.data_length = end;

        if self.section_length == 0 && self.data_length >= 3 {
            self.section_length = psi_section_length(&self.data);
        }

        self.section_length != 0
    }

    pub fn payload(&self) -> Option<&[u8]> {
        (self.section_length != 0 && self.data_length >= self.section_length)
            .then(|| &self.data[.. self.section_length])
    }

    /// Assembles PSI section from TS packets.
    /// Returns `Some(&[u8])` when PSI section is ready.
    pub fn assemble(&mut self, packet: &TsPacketRef) -> Option<&'_ [u8]> {
        let payload = packet.payload()?;
        let cc = packet.cc();

        if packet.is_payload_start() {
            let pointer_field = payload[0] as usize;
            let payload = &payload[1 ..];

            if pointer_field >= payload.len() {
                // Invalid pointer field
                self.clear();
                return None;
            }

            // Previous section + Start of new section
            if pointer_field > 0
                && self.append_data(cc, &payload[.. pointer_field])
                && self.data_length >= self.section_length
            {
                // Save new section start into self.head
                let tail = &payload[pointer_field ..];
                self.head_length = tail.len();
                self.head[.. self.head_length].copy_from_slice(tail);

                self.assembling = true;
                self.cc = cc;

                return Some(&self.data[.. self.section_length]);
            }

            // Start of new PSI section only
            let payload = &payload[pointer_field ..];
            let end = if payload.len() >= 3 {
                self.section_length = psi_section_length(payload);
                payload.len().min(self.section_length)
            } else {
                self.section_length = 0;
                payload.len()
            };
            self.data[.. end].copy_from_slice(&payload[.. end]);
            self.data_length = end;

            if self.section_length != 0 && self.data_length >= self.section_length {
                // PSI section is complete
                self.assembling = false;
                return Some(&self.data[.. self.section_length]);
            }

            self.assembling = true;
            self.cc = cc;
        } else if self.append_data(cc, payload) {
            if self.data_length >= self.section_length {
                // PSI section is complete
                self.assembling = false;
                return Some(&self.data[.. self.section_length]);
            }

            self.cc = cc;
        }

        None
    }

    #[inline]
    fn push(&mut self, payload: &[u8]) {
        self.buffer.extend_from_slice(payload);

        if self.size == 0 && self.buffer.len() >= 3 {
            self.size =
                3 + ((u16::from_be_bytes([self.buffer[1], self.buffer[2]]) & 0x0FFF) as usize);
        }
    }

    /// Mux TS packets into single PSI packet
    pub fn mux(&mut self, ts: &[u8]) {
        if !ts::is_payload(ts) {
            return;
        }

        let ts_offset = ts::get_payload_offset(ts) as usize;
        if ts_offset >= 188 {
            self.clear();
            return;
        }

        let cc = ts::get_cc(ts);

        if ts::is_pusi(ts) {
            let pointer_field = ts[ts_offset] as usize;
            if pointer_field >= 183 {
                self.clear();
                return;
            }
            let ts_offset = ts_offset + 1;

            if pointer_field == 0 || cc != (self.cc + 1) & 0x0F {
                self.clear();
            }

            // TODO: save pid into self.pid
            if self.buffer.is_empty() {
                self.push(&ts[ts_offset + pointer_field .. 188]);
                if self.size != 0 && self.buffer.len() > self.size {
                    self.buffer.resize(self.size, 0x00);
                }
            } else {
                if self.size != 0 && self.buffer.len() > self.size {
                    self.buffer.drain(0 .. self.size);
                    self.size = 0;
                }
                self.push(&ts[ts_offset .. 188]);
            }
        } else {
            if cc != (self.cc + 1) & 0x0F {
                self.clear();
                return;
            }

            self.push(&ts[ts_offset .. 188]);
            if self.buffer.len() > self.size {
                self.buffer.resize(self.size, 0x00);
            }
        }

        self.cc = cc;
    }

    /// Returns the PSI packet checksum
    #[inline]
    fn get_crc32(&self) -> u32 {
        let skip = self.size - 4;
        u32::from_be_bytes([
            self.buffer[skip],
            self.buffer[skip + 1],
            self.buffer[skip + 2],
            self.buffer[skip + 3],
        ])
    }

    /// Calculates the PSI packet checksum
    #[inline]
    fn calc_crc32(&self) -> u32 {
        let size = self.size - 4;
        crc32b(&self.buffer[.. size])
    }

    #[inline]
    fn check_crc32(&self) -> bool {
        self.get_crc32() == self.calc_crc32()
    }

    /// Returns `true` if buffer contains complete PSI packet
    #[inline]
    pub fn check(&self) -> bool {
        /* 3 - minimal PSI header, 4 - crc32 */
        self.size > 7 && self.buffer.len() >= self.size && self.check_crc32()
    }

    /// Finalize PSI packet. Push 4 bytes for CRC32, set PSI packet length,
    /// calculate CRC32.
    pub fn finalize(&mut self) {
        if self.size == 0 {
            self.buffer.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // place for CRC32
            self.size = self.buffer.len();

            let flags_and_len = (u16::from(self.buffer[1] & 0xF0) << 8) | ((self.size - 3) as u16);
            self.buffer[1 .. 3].copy_from_slice(&flags_and_len.to_be_bytes());
        }

        let skip = self.size - 4;
        let crc = crc32b(&self.buffer[.. skip]);
        self.buffer[skip .. skip + 4].copy_from_slice(&crc.to_be_bytes());
    }

    /// Convert PSI into TS packets
    /// Returns `true` while `ts` field contains valid TS packet
    ///
    /// # Examples
    ///
    /// ``` ignore
    /// use mpegts::ts::*;
    /// use mpegts::psi::*;
    ///
    /// psi.cc = 0;
    /// psi.pid = EIT_PID;
    /// let mut ts = Vec::<u8>::new()
    /// psi.demux(&mut ts);
    /// ```
    pub fn demux(&mut self, dst: &mut Vec<u8>) {
        let mut psi_skip = 0;
        let mut dst_skip = dst.len();

        let ts_count = (self.size + 1).div_ceil(184);
        dst.resize(dst_skip + 188 * ts_count, 0x00);

        while psi_skip < self.size {
            dst[dst_skip] = 0x47;
            ts::set_pid(&mut dst[dst_skip ..], self.pid);
            ts::set_payload_1(&mut dst[dst_skip ..]);
            ts::set_cc(&mut dst[dst_skip ..], self.cc);
            self.cc = (self.cc + 1) & 0x0F;

            let hdr_len = if psi_skip == 0 {
                ts::set_pusi_1(&mut dst[dst_skip ..]);
                5
            } else {
                4
            };
            dst_skip += hdr_len;

            let cpy_len = std::cmp::min(self.size - psi_skip, 188 - hdr_len);
            let dst_next = dst_skip + cpy_len;
            let psi_next = psi_skip + cpy_len;

            dst[dst_skip .. dst_next].copy_from_slice(&self.buffer[psi_skip .. psi_next]);

            dst_skip = dst_next;
            psi_skip = psi_next;
        }

        let remain = dst.len() - dst_skip;
        if remain > 0 {
            let dst_end = dst.len();
            dst[dst_skip .. dst_end].copy_from_slice(&ts::FILL_PACKET[.. remain]);
        }
    }
}

/// Trait for PSI to demux into TS packets
pub trait PsiDemux {
    /// Build list of PSI tables
    fn psi_list_assemble(&self) -> Vec<Psi>;

    /// Finalize
    fn finalize(&self, _psi: &mut Psi) {}

    /// Converts PSI into TS packets
    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        if psi_list.is_empty() {
            return;
        }

        let last_section_number = (psi_list.len() - 1) as u8;
        for (section_number, psi) in psi_list.iter_mut().enumerate() {
            psi.buffer[6] = section_number as u8;
            psi.buffer[7] = last_section_number;
            self.finalize(psi);
            psi.finalize();
            psi.pid = pid;
            psi.cc = *cc;
            psi.demux(dst);
            *cc = psi.cc;
        }
    }
}

#[inline]
fn psi_section_length(data: &[u8]) -> usize {
    3 + ((u16::from_be_bytes([data[1], data[2]]) & 0x0FFF) as usize)
}
