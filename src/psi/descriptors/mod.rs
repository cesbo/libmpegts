use std::fmt;
use std::slice::Iter;

mod raw; pub use psi::descriptors::raw::DescRaw;
mod _09; pub use psi::descriptors::_09::Desc09;
mod _0a; pub use psi::descriptors::_0a::{Desc0A, Desc0A_Item};
mod _0e; pub use psi::descriptors::_0e::Desc0E;
mod _40; pub use psi::descriptors::_40::Desc40;
mod _43; pub use psi::descriptors::_43::Desc43;
mod _44; pub use psi::descriptors::_44::Desc44;
mod _48; pub use psi::descriptors::_48::Desc48;
mod _4d; pub use psi::descriptors::_4d::Desc4D;
mod _4e; pub use psi::descriptors::_4e::Desc4E;
mod _52; pub use psi::descriptors::_52::Desc52;
mod _5a; pub use psi::descriptors::_5a::Desc5A;

/// Descriptors extends the definitions of programs and program elements.
#[derive(Debug)]
pub enum Descriptor {
    Desc09(Desc09),
    Desc0A(Desc0A),
    Desc0E(Desc0E),
    Desc40(Desc40),
    Desc43(Desc43),
    Desc44(Desc44),
    Desc48(Desc48),
    Desc4D(Desc4D),
    Desc4E(Desc4E),
    Desc52(Desc52),
    Desc5A(Desc5A),
    DescRaw(DescRaw)
}

impl Descriptor {
    fn parse(slice: &[u8]) -> Self {
        match slice[0] {
            0x09 if Desc09::check(slice) => Descriptor::Desc09(Desc09::parse(slice)),
            0x0A if Desc0A::check(slice) => Descriptor::Desc0A(Desc0A::parse(slice)),
            0x0E if Desc0E::check(slice) => Descriptor::Desc0E(Desc0E::parse(slice)),
            0x40 if Desc40::check(slice) => Descriptor::Desc40(Desc40::parse(slice)),
            0x43 if Desc43::check(slice) => Descriptor::Desc43(Desc43::parse(slice)),
            0x44 if Desc44::check(slice) => Descriptor::Desc44(Desc44::parse(slice)),
            0x48 if Desc48::check(slice) => Descriptor::Desc48(Desc48::parse(slice)),
            0x4D if Desc4D::check(slice) => Descriptor::Desc4D(Desc4D::parse(slice)),
            0x4E if Desc4E::check(slice) => Descriptor::Desc4E(Desc4E::parse(slice)),
            0x52 if Desc52::check(slice) => Descriptor::Desc52(Desc52::parse(slice)),
            0x5A if Desc5A::check(slice) => Descriptor::Desc5A(Desc5A::parse(slice)),
            _ => Descriptor::DescRaw(DescRaw::parse(slice)),
        }
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        match self {
            Descriptor::Desc09(v) => v.assemble(buffer),
            Descriptor::Desc0A(v) => v.assemble(buffer),
            Descriptor::Desc0E(v) => v.assemble(buffer),
            Descriptor::Desc40(v) => v.assemble(buffer),
            Descriptor::Desc43(v) => v.assemble(buffer),
            Descriptor::Desc44(v) => v.assemble(buffer),
            Descriptor::Desc48(v) => v.assemble(buffer),
            Descriptor::Desc4D(v) => v.assemble(buffer),
            Descriptor::Desc4E(v) => v.assemble(buffer),
            Descriptor::Desc52(v) => v.assemble(buffer),
            Descriptor::Desc5A(v) => v.assemble(buffer),
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
