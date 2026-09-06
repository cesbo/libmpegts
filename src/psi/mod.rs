mod cat;
mod descriptors;
mod eit;
mod error;
mod nit;
mod pat;
mod pmt;
mod sdt;
mod tdt;
mod tot;

pub use cat::*;
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
        NULL_PACKET,
        PACKET_SIZE,
        TsPacketMut,
        TsPacketRef,
    },
    utils::crc32b,
};

/// Collection of finalized PSI sections backed by a contiguous buffer.
#[derive(Debug, Clone, Default)]
pub struct Sections {
    buffer: Vec<u8>,
    starts: Vec<usize>,
}

impl Sections {
    pub(super) fn new(buffer: Vec<u8>, starts: Vec<usize>) -> Self {
        Self { buffer, starts }
    }

    /// Creates an empty collection to be filled with [`push_section`](Self::push_section).
    pub fn new_empty() -> Self {
        Self::default()
    }

    /// Appends one complete section (header through CRC, where the table has one).
    /// The bytes are copied as-is; the caller is responsible for their validity.
    pub fn push_section(&mut self, section: &[u8]) {
        self.starts.push(self.buffer.len());
        self.buffer.extend_from_slice(section);
    }

    /// Returns `true` if there are no sections.
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// Number of sections
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Removes all sections, keeping the allocated capacity.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.starts.clear();
    }

    /// First section, `None` when there are no sections.
    pub fn first(&self) -> Option<&[u8]> {
        (!self.is_empty()).then(|| &self[0])
    }

    /// Iterates over the sections in order.
    pub fn iter(&self) -> SectionsIter<'_> {
        SectionsIter {
            sections: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Sections {
    type Item = &'a [u8];
    type IntoIter = SectionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the sections of a [`Sections`], in order.
#[derive(Debug, Clone)]
pub struct SectionsIter<'a> {
    sections: &'a Sections,
    index: usize,
}

impl<'a> Iterator for SectionsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let section = self.sections.starts.get(self.index).map(|_| &self.sections[self.index]);
        self.index += usize::from(section.is_some());
        section
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.sections.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SectionsIter<'_> {}

/// One complete section, copied as-is like [`push_section`](Sections::push_section).
///
/// ```
/// use libmpegts::psi::{PsiPacketizer, Sections};
/// use libmpegts::ts::PACKET_SIZE;
///
/// let section = [0x70, 0x70, 0x05, 0xc0, 0x79, 0xcb, 0x12, 0x45]; // TDT
/// let mut packetizer = PsiPacketizer::new(0x14);
/// packetizer.set_sections(Sections::from(&section[..]));
/// let mut packet = [0u8; PACKET_SIZE];
/// assert!(packetizer.next(&mut packet));
/// assert_eq!(&packet[5 .. 13], &section);
/// ```
impl From<&[u8]> for Sections {
    fn from(section: &[u8]) -> Self {
        Self {
            buffer: section.to_vec(),
            starts: vec![0],
        }
    }
}

impl core::ops::Index<usize> for Sections {
    type Output = [u8];

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
///
/// Reassembles PSI sections from TS packets of one PID. A packet payload may
/// carry the end of a pending section followed by any number of complete
/// sections and at most one partial section; every section completed by an
/// [`assemble`](Self::assemble) call is available through
/// [`sections`](Self::sections).
///
/// ```
/// use libmpegts::{psi::{Psi, PmtSectionRef}, ts::TsPacketRef};
///
/// # let packets: Vec<[u8; 188]> = Vec::new();
/// let mut psi = Psi::default();
/// for packet in &packets {
///     psi.assemble(&TsPacketRef::from(packet));
///     for section in psi.sections() {
///         if let Ok(pmt) = PmtSectionRef::try_from(section) {
///             println!("program {}", pmt.program_number());
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Psi {
    /// Buffer for the section being assembled; section_length is at most
    /// 3 + 0x0fff = 4098, so any pending section fits
    data: [u8; 4098],
    data_length: usize,
    section_length: usize,
    assembling: bool,
    cc: u8,
    /// Sections completed by the most recent `assemble` call, in stream order
    sections: Sections,
}

impl Default for Psi {
    fn default() -> Psi {
        Psi {
            data: [0; 4098],
            data_length: 0,
            section_length: 0,
            assembling: false,
            cc: 0,
            sections: Sections::default(),
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

    /// Clears the PSI buffer, the completed sections and all fields
    pub fn clear(&mut self) {
        self.data_length = 0;
        self.section_length = 0;
        self.assembling = false;
        self.sections.clear();
    }

    /// Sections completed by the most recent [`assemble`](Self::assemble)
    /// call, in stream order. Empty when that call completed nothing.
    /// Iterable directly: `for section in psi.sections()`.
    pub fn sections(&self) -> &Sections {
        &self.sections
    }

    /// Appends continuation bytes to the pending section. Bytes beyond the
    /// section end are stuffing and ignored. Moves the section into the
    /// completed sections once complete.
    fn append_data(&mut self, mut payload: &[u8]) {
        if self.section_length == 0 {
            // Header split across packets: section_length is known once
            // 3 bytes are collected
            let n = (3 - self.data_length).min(payload.len());
            self.data[self.data_length .. self.data_length + n].copy_from_slice(&payload[.. n]);
            self.data_length += n;
            payload = &payload[n ..];

            if self.data_length < 3 {
                return;
            }
            self.section_length = psi_section_length(&self.data);
        }

        // section_length <= 4098 always fits data
        let n = (self.section_length - self.data_length).min(payload.len());
        self.data[self.data_length .. self.data_length + n].copy_from_slice(&payload[.. n]);
        self.data_length += n;

        if self.data_length >= self.section_length {
            self.sections.push_section(&self.data[.. self.section_length]);
            self.assembling = false;
        }
    }

    /// Parses back-to-back sections starting at a pointer_field position.
    /// Complete sections go to the completed sections; a trailing partial
    /// (down to a 1-byte header start) becomes the pending section.
    fn start_sections(&mut self, cc: u8, mut payload: &[u8]) {
        while let Some(&first) = payload.first() {
            if first == 0xff {
                // Stuffing: no more sections in this packet
                return;
            }

            if payload.len() >= 3 {
                let section_length = psi_section_length(payload);
                if section_length <= payload.len() {
                    self.sections.push_section(&payload[.. section_length]);
                    payload = &payload[section_length ..];
                    continue;
                }
            }

            // Partial section: always the last thing in a payload
            self.data[.. payload.len()].copy_from_slice(payload);
            self.data_length = payload.len();
            self.section_length = if payload.len() >= 3 {
                psi_section_length(payload)
            } else {
                0
            };
            self.assembling = true;
            self.cc = cc;
            return;
        }
    }

    /// Assembles PSI sections from TS packets.
    ///
    /// Processes the whole payload: the bytes before pointer_field finish the
    /// pending section, the bytes after it are parsed as back-to-back sections
    /// up to a 0xFF stuffing byte or a trailing partial. Every section
    /// completed by this call is available through
    /// [`sections`](Self::sections) until the next call. A continuity counter
    /// gap or a new section start drops an unfinished pending section.
    pub fn assemble(&mut self, packet: &TsPacketRef) {
        self.sections.clear();

        let Some(payload) = packet.payload() else {
            return;
        };
        let cc = packet.cc();
        let continuous = self.assembling && cc == (self.cc + 1) & 0x0f;

        if packet.is_payload_start() {
            let pointer_field = payload[0] as usize;
            let payload = &payload[1 ..];

            if pointer_field >= payload.len() {
                // Invalid pointer field
                self.clear();
                return;
            }

            if continuous {
                self.append_data(&payload[.. pointer_field]);
            }
            // A new section start ends the pending section either way
            self.assembling = false;

            self.start_sections(cc, &payload[pointer_field ..]);
        } else if continuous {
            self.append_data(payload);
            self.cc = cc;
        } else if self.assembling {
            // Continuity counter error
            self.clear();
        }
    }
}

pub(super) fn psi_section_length(data: &[u8]) -> usize {
    3 + ((u16::from_be_bytes([data[1], data[2]]) & 0x0fff) as usize)
}

/// Patches section_length, section numbers and CRC32 of generic-syntax
/// sections laid out back-to-back in `buffer`. Each section must end with
/// CRC32 placeholder bytes; header bits other than section_length are
/// preserved as written by the builder.
fn finalize_sections(mut buffer: Vec<u8>, starts: Vec<usize>) -> Sections {
    let last_section_number = (starts.len() - 1) as u8;

    for i in 0 .. starts.len() {
        let start = starts[i];
        let end = if i + 1 < starts.len() {
            starts[i + 1]
        } else {
            buffer.len()
        };

        // Patch section_length: total section bytes - 3
        let section_length = (end - start - 3) as u16;
        buffer[start + 1] = (buffer[start + 1] & 0xf0) | ((section_length >> 8) as u8 & 0x0f);
        buffer[start + 2] = section_length as u8;

        // Patch section_number and last_section_number
        buffer[start + 6] = i as u8;
        buffer[start + 7] = last_section_number;

        // Compute and write CRC32
        let crc = crc32b(&buffer[start .. end - PSI_CRC_SIZE]);
        buffer[end - PSI_CRC_SIZE .. end].copy_from_slice(&crc.to_be_bytes());
    }

    Sections::new(buffer, starts)
}

/// Section CRC32 size in bytes.
const PSI_CRC_SIZE: usize = 4;

/// Checks the trailing CRC32 of one complete PSI section.
///
/// `section` spans the generic 3-byte header through the CRC32, as yielded
/// by [`Psi::sections`].
/// Returns `false` for sections too short to carry a CRC32.
/// TDT carries no CRC32; TOT does despite being short-form.
pub fn check_crc32(section: &[u8]) -> bool {
    if section.len() < 3 + PSI_CRC_SIZE {
        return false;
    }

    let crc_offset = section.len() - PSI_CRC_SIZE;
    let p = &section[crc_offset ..];
    crc32b(&section[.. crc_offset]) == u32::from_be_bytes([p[0], p[1], p[2], p[3]])
}

/// Mutable borrowed PSI section for in-place patching.
///
/// Wraps one complete long-form section (fixed header through CRC32) and
/// carries the table-agnostic mutations: version and CRC32. Table-specific
/// wrappers add their own field setters on top (e.g.
/// [`PmtSectionMut`]).
///
/// After any field change call [`update_crc32`](Self::update_crc32) so the
/// section stays valid for CRC-checking consumers.
pub struct PsiSectionMut<'a>(&'a mut [u8]);

impl<'a> PsiSectionMut<'a> {
    /// Sets the 5-bit `version_number`, preserving the reserved bits and
    /// `current_next_indicator`.
    pub fn set_version(&mut self, version: u8) {
        self.0[5] = (self.0[5] & 0xc1) | ((version & 0x1f) << 1);
    }

    /// Recomputes the CRC32 over the section body and writes it into the
    /// last four bytes.
    pub fn update_crc32(&mut self) {
        let crc_offset = self.0.len() - PSI_CRC_SIZE;
        let crc = crc32b(&self.0[.. crc_offset]);
        self.0[crc_offset ..].copy_from_slice(&crc.to_be_bytes());
    }
}

impl AsRef<[u8]> for PsiSectionMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl AsMut<[u8]> for PsiSectionMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0
    }
}

impl<'a> TryFrom<&'a mut [u8]> for PsiSectionMut<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a mut [u8]) -> Result<Self, Self::Error> {
        // Fixed long-form header plus CRC32; a short-form section
        // (section_syntax_indicator clear) carries neither a version
        // nor a CRC32
        if value.len() < 8 + PSI_CRC_SIZE || (value[1] & 0x80) == 0 {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if psi_section_length(value) != value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        Ok(Self(value))
    }
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
    pub fn new(pid: u16) -> Self {
        Self {
            sections: Sections {
                buffer: Vec::new(),
                starts: Vec::new(),
            },
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
    pub fn is_empty(&self) -> bool {
        self.section_index >= self.sections.len()
    }

    /// PID the packets are emitted on.
    pub fn pid(&self) -> u16 {
        self.pid
    }

    /// Index of the section currently being packetized.
    /// Equals the section count when all sections are exhausted.
    pub fn section_index(&self) -> usize {
        self.section_index
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

#[cfg(test)]
mod tests {
    use super::*;

    fn section_with_crc(body: &[u8]) -> Vec<u8> {
        let mut section = body.to_vec();
        section.extend_from_slice(&crc32b(body).to_be_bytes());
        section
    }

    #[test]
    fn check_crc32_valid() {
        let section = section_with_crc(&[0x42, 0xf0, 0x10, 0x00, 0x01, 0xc3]);
        assert!(check_crc32(&section));
    }

    #[test]
    fn check_crc32_minimal_body() {
        let section = section_with_crc(&[0x42, 0xf0, 0x04]);
        assert!(check_crc32(&section));
    }

    #[test]
    fn check_crc32_corrupted_body() {
        let mut section = section_with_crc(&[0x42, 0xf0, 0x10, 0x00, 0x01, 0xc3]);
        section[4] ^= 0x01;
        assert!(!check_crc32(&section));
    }

    #[test]
    fn check_crc32_corrupted_crc() {
        let mut section = section_with_crc(&[0x42, 0xf0, 0x10, 0x00, 0x01, 0xc3]);
        let last = section.len() - 1;
        section[last] ^= 0x01;
        assert!(!check_crc32(&section));
    }

    #[test]
    fn check_crc32_too_short() {
        assert!(!check_crc32(&[]));
        assert!(!check_crc32(&[0x42, 0xf0]));
        assert!(!check_crc32(&[0x42, 0xf0, 0x00, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn push_section_roundtrip() {
        let first = section_with_crc(&[0x42, 0xf0, 0x10, 0x00, 0x01, 0xc3]);
        let second = section_with_crc(&[0x42, 0xf0, 0x04]);

        let mut sections = Sections::new_empty();
        assert!(sections.is_empty());
        sections.push_section(&first);
        sections.push_section(&second);

        assert_eq!(sections.len(), 2);
        assert_eq!(&sections[0], first.as_slice());
        assert_eq!(&sections[1], second.as_slice());
    }

    /// PAT section, payload of the tests/data PAT packet (40 bytes)
    const PAT: &[u8] = &[
        0x00, 0xb0, 0x25, 0x00, 0x01, 0xc3, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x10, 0x00, 0x01, 0xe4, 0x07,
        0x00, 0x02, 0xe4, 0x08, 0x00, 0x03, 0xe4, 0x09, 0x00, 0x04, 0xe4, 0x0a, 0x00, 0x05, 0xe4, 0x0b,
        0x00, 0x06, 0xe4, 0x0c, 0xb2, 0xd1, 0xb8, 0xde,
    ];

    /// PMT section, payload of the tests/data PMT packet (54 bytes)
    const PMT: &[u8] = &[
        0x02, 0xb0, 0x33, 0xc5, 0x17, 0xc3, 0x00, 0x00, 0xe9, 0x0e, 0xf0, 0x00, 0x02, 0xe9, 0x0e, 0xf0,
        0x0e, 0x0e, 0x03, 0xc1, 0x2e, 0xbc, 0x09, 0x04, 0x09, 0x63, 0xe5, 0x01, 0x52, 0x01, 0x01, 0x04,
        0xe9, 0x0f, 0xf0, 0x0e, 0x0e, 0x03, 0xc1, 0x2e, 0xbc, 0x0a, 0x04, 0x65, 0x6e, 0x67, 0x01, 0x52,
        0x01, 0x02, 0x06, 0x1b, 0x38, 0x6a,
    ];

    /// SDT section, payload of the two tests/data SDT packets (216 bytes)
    const SDT: &[u8] = &[
        0x42, 0xf0, 0xd5, 0x00, 0x01, 0xc3, 0x00, 0x00, 0x00, 0x01, 0xff, 0x00, 0x01, 0xfd, 0x80, 0x1d,
        0x48, 0x1b, 0x01, 0x06, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x12, 0x41, 0x76, 0x61, 0x6c, 0x70,
        0x61, 0x31, 0x3a, 0x20, 0x4d, 0x50, 0x45, 0x47, 0x32, 0x20, 0x4d, 0x48, 0x50, 0x00, 0x02, 0xfd,
        0x80, 0x1f, 0x48, 0x1d, 0x01, 0x06, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x14, 0x41, 0x76, 0x61,
        0x6c, 0x70, 0x61, 0x32, 0x3a, 0x20, 0x4d, 0x50, 0x45, 0x47, 0x32, 0x20, 0x4d, 0x48, 0x45, 0x47,
        0x35, 0x00, 0x03, 0xfd, 0x80, 0x1f, 0x48, 0x1d, 0x01, 0x06, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61,
        0x14, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x33, 0x3a, 0x20, 0x4d, 0x50, 0x45, 0x47, 0x32, 0x20,
        0x48, 0x42, 0x42, 0x54, 0x56, 0x00, 0x04, 0xfd, 0x80, 0x1d, 0x48, 0x1b, 0x01, 0x06, 0x41, 0x76,
        0x61, 0x6c, 0x70, 0x61, 0x12, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x34, 0x3a, 0x20, 0x4d, 0x50,
        0x45, 0x47, 0x32, 0x20, 0x54, 0x58, 0x54, 0x00, 0x05, 0xfd, 0x80, 0x18, 0x48, 0x16, 0x16, 0x06,
        0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x0d, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x35, 0x3a, 0x20,
        0x48, 0x32, 0x36, 0x34, 0x00, 0x06, 0xfd, 0x80, 0x1b, 0x48, 0x19, 0x19, 0x06, 0x41, 0x76, 0x61,
        0x6c, 0x70, 0x61, 0x10, 0x41, 0x76, 0x61, 0x6c, 0x70, 0x61, 0x36, 0x3a, 0x20, 0x48, 0x44, 0x20,
        0x48, 0x32, 0x36, 0x34, 0x51, 0x41, 0xe4, 0x75,
    ];

    /// TDT section, payload of the tests/data TDT packet: short-form, no
    /// CRC32 (8 bytes)
    const TDT: &[u8] = &[0x70, 0x70, 0x05, 0xe4, 0x7c, 0x18, 0x10, 0x12];

    /// RST section from tests/psi.rs `test_two_ts_two_psi`: exactly one TS
    /// payload after a zero pointer_field (183 bytes)
    const RST: &[u8] = &[
        0x71, 0x70, 0xb4, 0xe3, 0xc5, 0x22, 0x16, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xfa, 0x17, 0x4a, 0x1d,
    ];

    /// TS packet on PID 0x100 with `body` after the pointer_field (when
    /// `pointer` is set) and 0xFF stuffing to the end
    fn packet(cc: u8, pointer: Option<u8>, body: &[u8]) -> [u8; PACKET_SIZE] {
        let mut buf = [0xff; PACKET_SIZE];
        let mut ts = TsPacketMut::from(&mut buf);
        ts.init(0x100, cc);
        ts.set_payload();
        let payload = match pointer {
            Some(pointer) => {
                ts.set_pusi();
                let payload = ts.payload_mut().unwrap();
                payload[0] = pointer;
                &mut payload[1 ..]
            }
            None => ts.payload_mut().unwrap(),
        };
        payload[.. body.len()].copy_from_slice(body);
        buf
    }

    /// Feeds one packet and returns the sections it completed
    fn feed<'a>(psi: &'a mut Psi, packet: &[u8; PACKET_SIZE]) -> Vec<&'a [u8]> {
        psi.assemble(&TsPacketRef::from(packet));
        psi.sections().iter().collect()
    }

    #[test]
    fn sections_iter_first_and_clear() {
        let mut sections = Sections::new_empty();
        assert!(sections.first().is_none());
        assert_eq!(sections.iter().len(), 0);

        sections.push_section(PAT);
        sections.push_section(PMT);
        assert_eq!(sections.first(), Some(PAT));
        assert_eq!(sections.iter().len(), 2);
        assert_eq!(sections.iter().collect::<Vec<_>>(), [PAT, PMT]);
        let mut by_ref = Vec::new();
        for section in &sections {
            by_ref.push(section);
        }
        assert_eq!(by_ref, [PAT, PMT]);

        sections.clear();
        assert!(sections.is_empty());
        assert!(sections.first().is_none());
        assert_eq!(sections.iter().count(), 0);

        sections.push_section(TDT);
        assert_eq!(sections.iter().collect::<Vec<_>>(), [TDT]);
    }

    #[test]
    fn back_to_back_sections() {
        // Invariant: sections packed greedily - several per packet, a
        // pointer_field past the end of a pending section, a header split
        // 2 bytes before the packet end, stuffing where the encoder chose not
        // to split, a short-form section, a section ending in a continuation
        // packet - are each delivered exactly once, in order, byte-identical,
        // on the packet carrying their last byte
        let mut psi = Psi::default();

        // PAT + PMT + first 89 bytes of SDT
        let body = [PAT, PMT, &SDT[.. 89]].concat();
        assert_eq!(body.len(), 183);
        assert_eq!(feed(&mut psi, &packet(0, Some(0), &body)), [PAT, PMT]);
        assert!(psi.assembling);
        assert_eq!(psi.section_length, SDT.len());

        // rest of SDT (pointer_field 127) + PMT + 2 header bytes of PAT
        let body = [&SDT[89 ..], PMT, &PAT[.. 2]].concat();
        assert_eq!(body.len(), 183);
        assert_eq!(feed(&mut psi, &packet(1, Some(127), &body)), [SDT, PMT]);
        assert!(psi.assembling);
        assert_eq!(psi.section_length, 0);

        // rest of PAT (pointer_field 38) + PMT + PAT, then 51 bytes of stuffing
        let body = [&PAT[2 ..], PMT, PAT].concat();
        assert_eq!(feed(&mut psi, &packet(2, Some(38), &body)), [PAT, PMT, PAT]);
        assert!(!psi.assembling);

        // TDT + first 175 bytes of SDT
        let body = [TDT, &SDT[.. 175]].concat();
        assert_eq!(body.len(), 183);
        assert_eq!(feed(&mut psi, &packet(3, Some(0), &body)), [TDT]);

        // rest of SDT in a continuation packet, stuffing after it
        assert_eq!(feed(&mut psi, &packet(4, None, &SDT[175 ..])), [SDT]);
        assert_eq!(psi.sections().first(), Some(SDT));
        assert!(!psi.assembling);
    }

    #[test]
    fn exact_fit_section_delivered_on_its_packet() {
        // Invariant: a section whose last byte is the last payload byte is
        // delivered on that packet, not held until the next one; a following
        // PUSI packet after a continuity gap delivers only its own section
        assert_eq!(RST.len(), 183);

        let mut psi = Psi::default();
        assert_eq!(feed(&mut psi, &packet(0, Some(0), RST)), [RST]);
        assert!(!psi.assembling);

        assert_eq!(feed(&mut psi, &packet(5, Some(0), TDT)), [TDT]);
    }

    #[test]
    fn cc_gap_drops_partial_and_next_start_is_clean() {
        // Invariant: a continuity gap on a continuation packet drops the
        // pending partial and delivers nothing; the next PUSI packet starts
        // a fresh section that assembles normally
        let mut psi = Psi::default();
        assert!(feed(&mut psi, &packet(0, Some(0), &SDT[.. 183])).is_empty());
        // cc jumps 0 -> 2
        assert!(feed(&mut psi, &packet(2, None, &SDT[183 ..])).is_empty());
        assert!(!psi.assembling);
        // a further continuation is ignored while idle
        assert!(feed(&mut psi, &packet(3, None, &SDT[183 ..])).is_empty());

        assert!(feed(&mut psi, &packet(4, Some(0), &SDT[.. 183])).is_empty());
        assert_eq!(feed(&mut psi, &packet(5, None, &SDT[183 ..])), [SDT]);
    }

    #[test]
    fn duplicate_cc_drops_partial() {
        // Invariant: a repeated continuity counter is a gap, not a duplicate
        // to skip - on a continuation packet the partial is dropped, on a
        // PUSI packet the partial is dropped and the new start is parsed
        let mut psi = Psi::default();
        assert!(feed(&mut psi, &packet(0, Some(0), &SDT[.. 183])).is_empty());
        assert!(feed(&mut psi, &packet(0, None, &SDT[183 ..])).is_empty());
        assert!(!psi.assembling);

        assert!(feed(&mut psi, &packet(1, Some(0), &SDT[.. 183])).is_empty());
        // same cc as the previous packet: the 33 bytes before the pointer are
        // not appended, the PAT after the pointer is delivered
        let body = [&SDT[183 ..], PAT].concat();
        assert_eq!(feed(&mut psi, &packet(1, Some(33), &body)), [PAT]);
    }

    #[test]
    fn pointer_field_bounds() {
        // Invariant: a pointer_field beyond the payload returns None and
        // clears the pending partial; a pointer_field at the last payload
        // byte starts a section with a 1-byte header that completes across
        // the following packets, one of them a full 184-byte continuation
        let mut psi = Psi::default();
        assert!(feed(&mut psi, &packet(0, Some(0), &SDT[.. 183])).is_empty());
        assert!(feed(&mut psi, &packet(1, Some(183), &[])).is_empty());
        assert!(!psi.assembling);
        assert!(feed(&mut psi, &packet(2, None, &SDT[183 ..])).is_empty());

        let body = [&[0xff; 182][..], &SDT[.. 1]].concat();
        assert!(feed(&mut psi, &packet(3, Some(182), &body)).is_empty());
        assert!(psi.assembling);
        assert_eq!(psi.section_length, 0);
        assert!(feed(&mut psi, &packet(4, None, &SDT[1 .. 185])).is_empty());
        assert_eq!(psi.section_length, SDT.len());
        assert_eq!(feed(&mut psi, &packet(5, None, &SDT[185 ..])), [SDT]);
    }

    #[test]
    fn stuffing_only_payload_after_pusi() {
        // Invariant: PUSI with pointer 0 followed by 0xFF delivers nothing and
        // leaves the assembler idle; while a partial is pending it ends that
        // partial like any other section start
        let mut psi = Psi::default();
        assert!(feed(&mut psi, &packet(0, Some(0), &[])).is_empty());
        assert!(!psi.assembling);
        assert_eq!(psi.data_length, 0);

        assert!(feed(&mut psi, &packet(1, Some(0), &SDT[.. 183])).is_empty());
        assert!(feed(&mut psi, &packet(2, Some(0), &[])).is_empty());
        assert!(!psi.assembling);
        assert!(feed(&mut psi, &packet(3, None, &SDT[183 ..])).is_empty());
    }

    #[test]
    fn packetizer_section_index_tracks_boundaries() {
        // Two sections: a long one spanning 2 packets and a short one
        let long = section_with_crc(&vec![0x42; 200]);
        let short = section_with_crc(&[0x42, 0xf0, 0x04]);

        let mut sections = Sections::new_empty();
        sections.push_section(&long);
        sections.push_section(&short);

        let mut packetizer = PsiPacketizer::new(0x0011);
        assert_eq!(packetizer.pid(), 0x0011);
        packetizer.set_sections(sections);

        let mut packet = [0u8; PACKET_SIZE];
        assert_eq!(packetizer.section_index(), 0);
        assert!(packetizer.next(&mut packet));
        assert_eq!(packetizer.section_index(), 0);
        assert!(packetizer.next(&mut packet));
        assert_eq!(packetizer.section_index(), 1);
        assert!(packetizer.next(&mut packet));
        assert_eq!(packetizer.section_index(), 2);
        assert!(packetizer.is_empty());
        assert!(!packetizer.next(&mut packet));
    }
}
