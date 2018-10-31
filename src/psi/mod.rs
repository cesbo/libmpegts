use std::cmp;

use base::*;
use ts::*;
use crc32::*;

mod descriptors;
pub use psi::descriptors::*;

mod pat;
pub use psi::pat::*;

mod eit;
pub use psi::eit::*;

mod pmt;
pub use psi::pmt::*;

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

    cc: u8,
    skip: usize,
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
            cc: 0,
            skip: 0,
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
        self.skip = 0;
    }

    #[inline]
    fn push(&mut self, payload: &[u8]) {
        self.buffer.extend_from_slice(payload);

        if self.size == 0 && self.buffer.len() >= 3 {
            self.size = 3 + get_u12(&self.buffer[1..]) as usize;
        }
    }

    /// Mux TS packets into single PSI packet
    pub fn mux(&mut self, ts: &[u8]) {
        if ! is_payload(ts) {
            return;
        }

        let ts_offset = get_payload_offset(ts) as usize;
        if ts_offset >= 188 {
            self.clear();
            return;
        }

        let cc = get_cc(ts);

        if is_pusi(ts) {
            let pointer_field = ts[ts_offset] as usize;
            if pointer_field >= 183 {
                self.clear();
                return;
            }
            let ts_offset = ts_offset + 1;

            if pointer_field == 0 || cc != (self.cc + 1) & 0x0F {
                self.clear();
            }

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
        get_u32(&self.buffer[skip ..])
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
        set_u12(&mut self.buffer.as_mut_slice()[1 ..], (self.size as u16) - 3);

        let x = crc32b(&self.buffer[.. skip]);
        set_u32(&mut self.buffer.as_mut_slice()[skip ..], x);
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
    /// let mut ts = new_ts();
    /// set_pid(&mut ts, EIT_PID);
    /// while psi.demux(&mut ts) {
    ///     ...
    /// }
    /// ```
    pub fn demux(&mut self, ts: &mut [u8]) -> bool {
        if self.skip == self.size {
            self.skip = 0;
            return false;
        }

        let ts_skip = {
            if self.skip > 183 {
                4
            } else if self.skip == 0 {
                set_payload_1(ts);
                set_pusi_1(ts);
                ts[4] = 0x00;
                5
            } else /* if self.skip == 183 */ {
                set_pusi_0(ts);
                4
            }
        };

        let cc = get_cc(ts);
        set_cc(ts, cc + 1);

        let len = cmp::min(self.size - self.skip, 188 - ts_skip);
        let next = self.skip + len;
        let ts_end = ts_skip + len;
        ts[ts_skip .. ts_end].copy_from_slice(&self.buffer[self.skip .. next]);

        self.skip = next;
        if self.skip == self.size && ts_end != 188 {
            ts[ts_end .. 188].copy_from_slice(&FILL_PACKET[ts_end .. 188]);
        }

        true
    }
}
