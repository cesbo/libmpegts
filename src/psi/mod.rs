use base;
use ts;
use crc32::*;

mod descriptors;
pub use psi::descriptors::*;

mod pat;
pub use psi::pat::*;

mod eit;
pub use psi::eit::*;

mod pmt;
pub use psi::pmt::*;

mod nit;
pub use psi::nit::*;

mod sdt;
pub use psi::sdt::*;

/// Program Specific Information includes normative data which is necessary for
/// the demultiplexing of transport streams and the successful regeneration of
/// programs.
///
/// # Fields
///
/// * `buffer` - buffer for PSI packet
/// * `size` - actual PSI size
///
/// size of the buffer could be more than actual PSI size as contains TS stuffing
/// bytes or part of the next PSI.
#[derive(Debug)]
pub struct Psi {
    pub buffer: Vec<u8>,
    pub size: usize, // PSI size

    pub pid: u16,
    pub cc: u8,
}

impl Default for Psi {
    fn default() -> Psi {
        Psi {
            buffer: {
                let mut buffer: Vec<u8> = Vec::new();
                buffer.reserve(4095 + 184);
                buffer
            },
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
    /// Clears the PSI buffer and all fields
    fn clear(&mut self) {
        self.buffer.clear();
        self.size = 0;
    }

    #[inline]
    fn push(&mut self, payload: &[u8]) {
        self.buffer.extend_from_slice(payload);

        if self.size == 0 && self.buffer.len() >= 3 {
            self.size = 3 + base::get_u12(&self.buffer[1..]) as usize;
        }
    }

    /// Mux TS packets into single PSI packet
    pub fn mux(&mut self, ts: &[u8]) {
        if ! ts::is_payload(ts) {
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
        let skip = self.size as usize - 4;
        base::get_u32(&self.buffer[skip ..])
    }

    /// Calculates the PSI packet checksum
    #[inline]
    fn calc_crc32(&self) -> u32 {
        let size = self.size as usize - 4;
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
        self.size > 7 &&
            self.buffer.len() >= self.size &&
            self.check_crc32()
    }

    /// Returns the PSI packet version
    #[inline]
    pub fn get_version(&self) -> u8 {
        (self.buffer[5] & 0x3E) >> 1
    }

    /// Sets the PSI packet version
    #[inline]
    pub fn set_version(&mut self, version: u8) {
        self.buffer[5] = 0xC0 | ((version << 1) & 0x3E) | 0x01;
    }

    /// Init PSI packet. Push into buffer 3 bytes: table_id and
    /// PSI packet length.
    pub fn init(&mut self, table_id: u8) {
        self.clear();
        self.buffer.resize(3, 0x00);
        self.buffer[0] = table_id;
        self.buffer[1] = 0xB0;
    }

    /// Finalize PSI packet. Push 4 bytes for CRC32, set PSI packet length,
    /// calculate CRC32.
    pub fn finalize(&mut self) {
        let skip = self.buffer.len();
        self.buffer.resize(skip + 4, 0x00);

        self.size = self.buffer.len();
        base::set_u12(&mut self.buffer[1 ..], (self.size as u16) - 3);

        let x = crc32b(&self.buffer[.. skip]);
        base::set_u32(&mut self.buffer[skip ..], x);
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

        let ts_count = (self.size + 1 + 183) / 184;
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

/// Trait for PSI items
pub trait PsiDemuxItem {
    /// Converts PSI item into the bytes
    fn assemble(&self, dst: &mut Vec<u8>);
    /// Returns size required for assembled item.
    /// Used to compare with `PsiDemux::psi_max_size()` before assembling
    fn size(&self) -> usize;
}

/// Trait for PSI to demux into TS packets
pub trait PsiDemux {
    /// Build list of PSI tables
    fn psi_list_assemble(&self) -> Vec<Psi>;

    /// Converts PSI into TS packets
    fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = self.psi_list_assemble();
        let mut section_number: u8 = 0;
        let last_section_number = (psi_list.len() - 1) as u8;
        for psi in &mut psi_list {
            psi.buffer[6] = section_number;
            psi.buffer[7] = last_section_number;
            psi.finalize();

            section_number += 1;

            psi.pid = pid;
            psi.cc = *cc;
            psi.demux(dst);
            *cc = psi.cc;
        }
    }
}
