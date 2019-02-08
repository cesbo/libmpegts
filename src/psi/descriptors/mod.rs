use std::fmt;
use std::slice::Iter;

mod raw; pub use raw::DescRaw;
mod x09; pub use x09::Desc09;
mod x0a; pub use x0a::Desc0A;
mod x0e; pub use x0e::Desc0E;
mod x40; pub use x40::Desc40;
mod x41; pub use x41::Desc41;
mod x43; pub use x43::Desc43;
mod x44; pub use x44::Desc44;
mod x48; pub use x48::Desc48;
mod x4d; pub use x4d::Desc4D;
mod x4e; pub use x4e::Desc4E;
mod x52; pub use x52::Desc52;
mod x5a; pub use x5a::Desc5A;
mod x83; pub use x83::Desc83;

/// Descriptors extends the definitions of programs and program elements.
#[derive(Debug)]
pub enum Descriptor {
    Desc09(Desc09),
    Desc0A(Desc0A),
    Desc0E(Desc0E),
    Desc40(Desc40),
    Desc41(Desc41),
    Desc43(Desc43),
    Desc44(Desc44),
    Desc48(Desc48),
    Desc4D(Desc4D),
    Desc4E(Desc4E),
    Desc52(Desc52),
    Desc5A(Desc5A),
    Desc83(Desc83),
    DescRaw(DescRaw)
}

impl Descriptor {
    /// Validates descriptor length with ::check(slice) and parse
    fn parse(slice: &[u8]) -> Self {
        match slice[0] {
            0x09 if Desc09::check(slice) => Descriptor::Desc09(Desc09::parse(slice)),
            0x0A if Desc0A::check(slice) => Descriptor::Desc0A(Desc0A::parse(slice)),
            0x0E if Desc0E::check(slice) => Descriptor::Desc0E(Desc0E::parse(slice)),
            0x40 if Desc40::check(slice) => Descriptor::Desc40(Desc40::parse(slice)),
            0x41 if Desc41::check(slice) => Descriptor::Desc41(Desc41::parse(slice)),
            0x43 if Desc43::check(slice) => Descriptor::Desc43(Desc43::parse(slice)),
            0x44 if Desc44::check(slice) => Descriptor::Desc44(Desc44::parse(slice)),
            0x48 if Desc48::check(slice) => Descriptor::Desc48(Desc48::parse(slice)),
            0x4D if Desc4D::check(slice) => Descriptor::Desc4D(Desc4D::parse(slice)),
            0x4E if Desc4E::check(slice) => Descriptor::Desc4E(Desc4E::parse(slice)),
            0x52 if Desc52::check(slice) => Descriptor::Desc52(Desc52::parse(slice)),
            0x5A if Desc5A::check(slice) => Descriptor::Desc5A(Desc5A::parse(slice)),
            0x83 if Desc83::check(slice) => Descriptor::Desc83(Desc83::parse(slice)),
            _ => Descriptor::DescRaw(DescRaw::parse(slice)),
        }
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        match self {
            Descriptor::Desc09(v) => v.assemble(buffer),
            Descriptor::Desc0A(v) => v.assemble(buffer),
            Descriptor::Desc0E(v) => v.assemble(buffer),
            Descriptor::Desc40(v) => v.assemble(buffer),
            Descriptor::Desc41(v) => v.assemble(buffer),
            Descriptor::Desc43(v) => v.assemble(buffer),
            Descriptor::Desc44(v) => v.assemble(buffer),
            Descriptor::Desc48(v) => v.assemble(buffer),
            Descriptor::Desc4D(v) => v.assemble(buffer),
            Descriptor::Desc4E(v) => v.assemble(buffer),
            Descriptor::Desc52(v) => v.assemble(buffer),
            Descriptor::Desc5A(v) => v.assemble(buffer),
            Descriptor::Desc83(v) => v.assemble(buffer),
            Descriptor::DescRaw(v) => v.assemble(buffer)
        };
    }

    fn size(&self) -> usize {
        match self {
            Descriptor::Desc09(v) => v.size(),
            Descriptor::Desc0A(v) => v.size(),
            Descriptor::Desc0E(v) => v.size(),
            Descriptor::Desc40(v) => v.size(),
            Descriptor::Desc41(v) => v.size(),
            Descriptor::Desc43(v) => v.size(),
            Descriptor::Desc44(v) => v.size(),
            Descriptor::Desc48(v) => v.size(),
            Descriptor::Desc4D(v) => v.size(),
            Descriptor::Desc4E(v) => v.size(),
            Descriptor::Desc52(v) => v.size(),
            Descriptor::Desc5A(v) => v.size(),
            Descriptor::Desc83(v) => v.size(),
            Descriptor::DescRaw(v) => v.size()
        }
    }
}


macro_rules! impl_into_descriptor {
    ( $( $d:tt ),* ) => {
        $( impl Into<Descriptor> for $d {
            fn into(self) -> Descriptor {
                Descriptor::$d(self)
            }
        } )*
    };
}

impl_into_descriptor!(Desc09, Desc0A, Desc0E, Desc40, Desc41, Desc43, Desc44, Desc48, Desc4D, Desc4E, Desc52, Desc5A, Desc83, DescRaw);

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

    pub fn assemble(&self, buffer: &mut Vec<u8>) -> usize {
        let size = buffer.len();
        for item in &self.0 {
            item.assemble(buffer);
        }
        buffer.len() - size
    }

    pub fn size(&self) -> usize {
        let mut x = 0;
        for item in &self.0 {
            x += item.size();
        }
        x
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
    pub fn push<T: Into<Descriptor>>(&mut self, desc: T) {
        self.0.push(desc.into());
    }

    #[inline]
    pub fn iter(&self) -> Iter<Descriptor> {
        self.0.iter()
    }
}
