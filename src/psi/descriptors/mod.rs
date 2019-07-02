use std::{
    fmt,
    any::Any,
};

mod raw; pub use raw::*;
mod x09; pub use x09::*;
mod x0a; pub use x0a::*;
mod x0e; pub use x0e::*;
mod x40; pub use x40::*;
mod x41; pub use x41::*;
mod x43; pub use x43::*;
mod x44; pub use x44::*;
mod x48; pub use x48::*;
mod x4d; pub use x4d::*;
mod x4e; pub use x4e::*;
mod x52; pub use x52::*;
mod x58; pub use x58::*;
mod x5a; pub use x5a::*;
mod x83; pub use x83::*;


pub trait AsAny {
    fn as_any_ref(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}


pub trait Desc: AsAny + fmt::Debug {
    fn tag(&self) -> u8;
    fn size(&self) -> usize;
    fn assemble(&self, buffer: &mut Vec<u8>);
}


impl<T: 'static + Desc> AsAny for T {
    fn as_any_ref(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}


/// Descriptors extends the definitions of programs and program elements.
pub struct Descriptor(Box<dyn Desc>);


impl<T: 'static + Desc> From<T> for Descriptor {
    fn from(desc: T) -> Self { Descriptor(Box::new(desc)) }
}


impl fmt::Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt(f) }
}


impl Descriptor {
    /// Validates descriptor length with ::check(slice) and parse
    fn parse(slice: &[u8]) -> Self {
        match slice[0] {
            0x09 if Desc09::check(slice) => Desc09::parse(slice).into(),
            0x0A if Desc0A::check(slice) => Desc0A::parse(slice).into(),
            0x0E if Desc0E::check(slice) => Desc0E::parse(slice).into(),
            0x40 if Desc40::check(slice) => Desc40::parse(slice).into(),
            0x41 if Desc41::check(slice) => Desc41::parse(slice).into(),
            0x43 if Desc43::check(slice) => Desc43::parse(slice).into(),
            0x44 if Desc44::check(slice) => Desc44::parse(slice).into(),
            0x48 if Desc48::check(slice) => Desc48::parse(slice).into(),
            0x4D if Desc4D::check(slice) => Desc4D::parse(slice).into(),
            0x4E if Desc4E::check(slice) => Desc4E::parse(slice).into(),
            0x52 if Desc52::check(slice) => Desc52::parse(slice).into(),
            0x58 if Desc58::check(slice) => Desc58::parse(slice).into(),
            0x5A if Desc5A::check(slice) => Desc5A::parse(slice).into(),
            0x83 if Desc83::check(slice) => Desc83::parse(slice).into(),
            _ => DescRaw::parse(slice).into(),
        }
    }

    #[inline]
    fn assemble(&self, buffer: &mut Vec<u8>) { self.0.assemble(buffer) }

    #[inline]
    fn size(&self) -> usize { self.0.size() }

    #[inline]
    pub fn tag(&self) -> u8 { self.0.tag() }

    #[inline]
    pub fn downcast_ref<T: 'static + Desc>(&self) -> &T {
        self.0.as_any_ref().downcast_ref::<T>().unwrap()
    }

    #[inline]
    pub fn downcast_mut<T: 'static + Desc>(&mut self) -> &mut T {
        self.0.as_any_mut().downcast_mut::<T>().unwrap()
    }
}


/// Array of descriptors
#[derive(Default)]
pub struct Descriptors(Vec<Descriptor>);


impl fmt::Debug for Descriptors {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt(f) }
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

    #[inline]
    pub fn size(&self) -> usize { self.0.iter().fold(0, |acc, x| acc + x.size()) }

    #[inline]
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    #[inline]
    pub fn len(&self) -> usize { self.0.len() }

    #[inline]
    pub fn get(&mut self, index: usize) -> Option<&Descriptor> { self.0.get(index) }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Descriptor> { self.0.get_mut(index) }

    #[inline]
    pub fn push<T>(&mut self, desc: T)
    where
        T: Into<Descriptor>,
    {
        self.0.push(desc.into())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Descriptor> { self.0.iter() }
}
