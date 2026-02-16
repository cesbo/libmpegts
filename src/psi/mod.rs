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

use crate::ts::{
    NULL_PACKET,
    PACKET_SIZE,
    TsPacketMut,
    TsPacketRef,
};

/// Collection of finalized PSI sections backed by a contiguous buffer.
pub struct Sections {
    buffer: Vec<u8>,
    starts: Vec<usize>,
}

impl Sections {
    pub(super) fn new(buffer: Vec<u8>, starts: Vec<usize>) -> Self {
        Self { buffer, starts }
    }

    /// Returns `true` if there are no sections.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// Number of sections
    #[inline]
    pub fn len(&self) -> usize {
        self.starts.len()
    }
}

impl core::ops::Index<usize> for Sections {
    type Output = [u8];

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let start = self.starts[index];
        let end = if index + 1 < self.starts.len() {
            self.starts[index + 1]
        } else {
            self.buffer.len()
        };
        &self.buffer[start .. end]
    }
}

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
    cc: u8,
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
            cc: 0,
        }
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
        psi.data[0] = table_id;
        psi.data[1] = 0xb0;
        psi.data[2] = 0x00;
        psi
    }

    /// Clears the PSI buffer and all fields
    pub fn clear(&mut self) {
        self.data_length = 0;
        self.head_length = 0;
        self.section_length = 0;
        self.assembling = false;
    }

    /// Appends data to current section being assembled.
    /// Returns `true` if section_length is known (header received).
    fn append_data(&mut self, cc: u8, payload: &[u8]) -> bool {
        if !self.assembling {
            return false;
        }

        if cc != (self.cc + 1) & 0x0f {
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
}

#[inline]
pub(super) fn psi_section_length(data: &[u8]) -> usize {
    3 + ((u16::from_be_bytes([data[1], data[2]]) & 0x0fff) as usize)
}

/// Packetizes PSI [`Sections`] into MPEG-TS packets.
///
/// Owns `Sections` and produces one TS packet per [`next`](Self::next) call
/// into a caller-provided buffer. Continuity counter persists across
/// [`reset`](Self::reset) calls for periodic re-transmission.
pub struct PsiPacketizer {
    sections: Sections,
    pid: u16,
    cc: u8,
    section_index: usize,
    offset: usize,
}

impl PsiPacketizer {
    /// Creates a new packetizer for the given PID and sections.
    pub fn new(pid: u16, sections: Sections) -> Self {
        Self {
            sections,
            pid,
            cc: 0,
            section_index: 0,
            offset: 0,
        }
    }

    /// Replaces sections and resets position.
    /// Continuity counter is preserved for CC continuity across version changes.
    pub fn set_sections(&mut self, sections: Sections) {
        self.sections = sections;
        self.section_index = 0;
        self.offset = 0;
    }

    /// Resets position to the beginning of sections.
    /// Continuity counter is preserved for periodic re-transmission.
    pub fn reset(&mut self) {
        self.section_index = 0;
        self.offset = 0;
    }

    /// Returns `true` if all sections have been packetized.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.section_index >= self.sections.len()
    }

    /// Writes the next TS packet into `packet`.
    /// Returns `true` if a packet was written, `false` when all sections are exhausted.
    pub fn next(&mut self, packet: &mut [u8; PACKET_SIZE]) -> bool {
        if self.section_index >= self.sections.len() {
            return false;
        }

        let section = &self.sections[self.section_index];

        let mut packet = TsPacketMut::from(packet);
        packet.init(self.pid, self.cc);
        packet.set_payload();

        self.cc = (self.cc + 1) & 0x0F;

        let payload = if self.offset == 0 {
            // First packet of section
            packet.set_pusi();
            let payload = packet.payload_mut().unwrap();
            // pointer_field
            payload[0] = 0x00;
            &mut payload[1 ..]
        } else {
            // Continuation packet
            packet.payload_mut().unwrap()
        };

        let available = payload.len();
        let remaining = section.len() - self.offset;
        let to_copy = available.min(remaining);
        payload[.. to_copy].copy_from_slice(&section[self.offset .. self.offset + to_copy]);
        self.offset += to_copy;

        // stuffing bytes
        if available > to_copy {
            let stuffing = &NULL_PACKET.as_ref()[4 ..];
            payload[to_copy .. available].copy_from_slice(&stuffing[to_copy .. available]);
        }

        if self.offset >= section.len() {
            self.section_index += 1;
            self.offset = 0;
        }

        true
    }
}
