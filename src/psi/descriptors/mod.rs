use super::PsiSectionError;

mod x09;
mod x0a;
mod x40;
mod x41;
mod x43;
mod x44;
mod x48;
mod x4d;
mod x4e;
mod x52;
mod x53;
mod x54;
mod x55;
mod x56;
mod x58;
mod x59;
mod x5a;
mod x5b;
mod x5f;
mod x83;

pub use x09::*;
pub use x0a::*;
pub use x40::*;
pub use x41::*;
pub use x43::*;
pub use x44::*;
pub use x48::*;
pub use x4d::*;
pub use x4e::*;
pub use x52::*;
pub use x53::*;
pub use x54::*;
pub use x55::*;
pub use x56::*;
pub use x58::*;
pub use x59::*;
pub use x5a::*;
pub use x5b::*;
pub use x5f::*;
pub use x83::*;

/// Encoder for one descriptor.
pub trait Descriptor {
    /// Appends tag, length and payload bytes to `dst`.
    ///
    /// Returns [`PsiSectionError::InvalidDescriptorLength`] when the payload
    /// does not fit the 8-bit descriptor length.
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError>;
}

/// Reference to a single MPEG-TS descriptor (tag + length + data).
#[derive(Debug, Clone, Copy)]
pub struct DescriptorRef<'a>(&'a [u8]);

impl<'a> DescriptorRef<'a> {
    pub fn tag(&self) -> u8 {
        self.0[0]
    }

    pub fn data(&self) -> &'a [u8] {
        &self.0[2 ..]
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.0
    }
}

/// Reference to a list of MPEG-TS descriptors.
pub struct DescriptorsRef<'a>(&'a [u8]);

impl<'a> From<&'a [u8]> for DescriptorsRef<'a> {
    fn from(value: &'a [u8]) -> Self {
        DescriptorsRef(value)
    }
}

impl<'a> IntoIterator for DescriptorsRef<'a> {
    type Item = Result<DescriptorRef<'a>, PsiSectionError>;
    type IntoIter = DescriptorIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DescriptorIter {
            data: self.0,
            offset: 0,
        }
    }
}

/// Iterator over MPEG-TS descriptors.
pub struct DescriptorIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for DescriptorIter<'a> {
    type Item = Result<DescriptorRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        if self.offset + 2 > self.data.len() {
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }
        let end = self.offset + 2 + self.data[self.offset + 1] as usize;
        if end > self.data.len() {
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }
        let desc = DescriptorRef(&self.data[self.offset .. end]);
        self.offset = end;
        Some(Ok(desc))
    }
}
