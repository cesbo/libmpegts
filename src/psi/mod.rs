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

use crate::ts::TsPacketRef;

/// Collection of finalized PSI sections backed by a contiguous buffer.
/// Provides zero-copy access to individual section slices.
pub struct Sections<'a> {
    buffer: &'a [u8],
    starts: &'a [usize],
    total: usize,
}

impl<'a> Sections<'a> {
    pub(super) fn new(buffer: &'a [u8], starts: &'a [usize]) -> Self {
        Self {
            buffer,
            starts,
            total: starts.len(),
        }
    }

    /// Number of sections
    #[inline]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns `true` if there are no sections
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Iterator over section slices
    pub fn iter(&self) -> SectionsIter<'a> {
        SectionsIter {
            buffer: self.buffer,
            starts: self.starts,
            total: self.total,
            index: 0,
        }
    }

    fn section_slice(&self, index: usize) -> &'a [u8] {
        let start = self.starts[index];
        let end = if index + 1 < self.total {
            self.starts[index + 1]
        } else {
            self.buffer.len()
        };
        &self.buffer[start..end]
    }
}

impl<'a> core::ops::Index<usize> for Sections<'a> {
    type Output = [u8];

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.section_slice(index)
    }
}

impl<'a> IntoIterator for &Sections<'a> {
    type Item = &'a [u8];
    type IntoIter = SectionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over section slices in a [`Sections`] collection
pub struct SectionsIter<'a> {
    buffer: &'a [u8],
    starts: &'a [usize],
    total: usize,
    index: usize,
}

impl<'a> Iterator for SectionsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.total {
            return None;
        }
        let start = self.starts[self.index];
        let end = if self.index + 1 < self.total {
            self.starts[self.index + 1]
        } else {
            self.buffer.len()
        };
        self.index += 1;
        Some(&self.buffer[start..end])
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for SectionsIter<'a> {}

pub struct Descriptor {
    pub tag: u8,
    pub data: Vec<u8>,
}

pub struct ElementaryStream {
    pub stream_type: u8,

    /// ES_info descriptors
    pub pmt_descriptors: Vec<Descriptor>,
}

pub struct Service {
    /// Service ID (PNR)
    pub service_id: u16,
    /// PMT PID and Starting PID for the service:
    /// Video and PCR = PMT PID + 1
    /// Audio = PMT PID + 2
    pub pid: u16,

    /// PMT Descriptors
    pub pmt_descriptors: Vec<Descriptor>,
    /// SDT Descriptors
    pub sdt_descriptors: Vec<Descriptor>,

    pub streams: Vec<ElementaryStream>,
}

pub struct PsiConfig {
    pub version: u8,
    pub transport_stream_id: u16,
    pub original_network_id: u16,
    pub services: Vec<Service>,
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
