use std::fmt;
use std::slice::Iter;

mod raw; pub use psi::descriptors::raw::*;
mod _4d; pub use psi::descriptors::_4d::*;
mod _4e; pub use psi::descriptors::_4e::*;
mod _48; pub use psi::descriptors::_48::*;

/// Descriptors extends the definitions of programs and program elements.
#[derive(Debug)]
pub enum Descriptor {
    Desc4D(Desc4D),
    Desc4E(Desc4E),
    Desc48(Desc48),
    DescRaw(DescRaw)
}

impl Descriptor {
    fn parse(slice: &[u8]) -> Self {
        match slice[0] {
            0x4D if Desc4D::check(slice) => Descriptor::Desc4D(Desc4D::parse(slice)),
            0x4E if Desc4E::check(slice) => Descriptor::Desc4E(Desc4E::parse(slice)),
            0x48 if Desc48::check(slice) => Descriptor::Desc48(Desc48::parse(slice)),
            _ => Descriptor::DescRaw(DescRaw::parse(slice)),
        }
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        match self {
            Descriptor::Desc4D(v) => v.assemble(buffer),
            Descriptor::Desc4E(v) => v.assemble(buffer),
            Descriptor::Desc48(v) => v.assemble(buffer),
            Descriptor::DescRaw(v) => v.assemble(buffer)
        };
    }
}

/// Array of descriptors
#[derive(Default)]
pub struct Descriptors(Vec<Descriptor>);

impl fmt::Debug for Descriptors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Descriptors {
    pub fn parse(&mut self, slice: &[u8]) {
        let mut skip: usize = 0;
        while slice.len() >= skip + 2 {
            let next = skip + 2 + slice[skip + 1] as usize;
            if next > slice.len() {
                break;
            }
            self.0.push(Descriptor::parse(&slice[skip .. next]));
            skip = next;
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        for item in &self.0 {
            item.assemble(buffer);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn push(&mut self, desc: Descriptor) {
        self.0.push(desc);
    }

    #[inline]
    pub fn iter(&self) -> Iter<Descriptor> {
        self.0.iter()
    }
}
