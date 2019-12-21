// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use std::{
    fmt,
    convert::TryFrom,
};

use bitwrap::{
    BitWrap,
    BitWrapError,
};

mod x09; pub use x09::*;
mod x0a; pub use x0a::*;
mod x0e; pub use x0e::*;
mod x40; pub use x40::*;
mod x41; pub use x41::*;
mod x43; pub use x43::*;
mod x44; pub use x44::*;
// mod x48; pub use x48::*;
// mod x4d; pub use x4d::*;
// mod x4e; pub use x4e::*;
mod x52; pub use x52::*;
mod x58; pub use x58::*;
mod x5a; pub use x5a::*;
mod x83; pub use x83::*;


/// Descriptors extends the definitions of programs and program elements.
#[derive(Clone, Debug)]
pub enum Descriptor {
    Desc09(Desc09),
    Desc0A(Desc0A),
    Desc0E(Desc0E),
    Desc40(Desc40),
    Desc41(Desc41),
    Desc43(Desc43),
    Desc44(Desc44),
    // Desc48(Desc48),
    // Desc4D(Desc4D),
    // Desc4E(Desc4E),
    Desc52(Desc52),
    Desc58(Desc58),
    Desc5A(Desc5A),
    Desc83(Desc83),
    DescRaw(Vec<u8>),
}


macro_rules! into_descriptor {
    ( $( $desc: ident, )* ) => {
        $(
            impl From<$desc> for Descriptor {
                #[inline]
                fn from(desc: $desc) -> Self { Descriptor::$desc(desc) }
            }
        )*
    };
}

into_descriptor! [
    Desc09,
    Desc0A,
    Desc0E,
    Desc40,
    Desc41,
    Desc43,
    Desc44,
    Desc52,
    Desc58,
    Desc5A,
    Desc83,
];


impl TryFrom<&[u8]> for Descriptor {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let desc: Descriptor = match value[0] {
            0x09 => Desc09::try_from(value)?.into(),
            0x0A => Desc0A::try_from(value)?.into(),
            0x0E => Desc0E::try_from(value)?.into(),
            0x40 => Desc40::try_from(value)?.into(),
            0x41 => Desc41::try_from(value)?.into(),
            0x43 => Desc43::try_from(value)?.into(),
            0x44 => Desc44::try_from(value)?.into(),
            // 0x48 => Desc48::parse(value).into(),
            // 0x4D => Desc4D::parse(value).into(),
            // 0x4E => Desc4E::parse(value).into(),
            0x52 => Desc52::try_from(value)?.into(),
            0x58 => Desc58::try_from(value)?.into(),
            0x5A => Desc5A::try_from(value)?.into(),
            0x83 => Desc83::try_from(value)?.into(),
            _ => Descriptor::DescRaw(value.into()),
        };

        Ok(desc)
    }
}


impl Descriptor {
    fn assemble(&self, buffer: &mut Vec<u8>) {
        match self {
            Descriptor::Desc09(v) => v.assemble(buffer),
            Descriptor::Desc0A(v) => v.assemble(buffer),
            Descriptor::Desc0E(v) => v.assemble(buffer),
            Descriptor::Desc40(v) => v.assemble(buffer),
            Descriptor::Desc41(v) => v.assemble(buffer),
            Descriptor::Desc43(v) => v.assemble(buffer),
            Descriptor::Desc44(v) => v.assemble(buffer),
            // Descriptor::Desc48(v) => v.assemble(buffer),
            // Descriptor::Desc4D(v) => v.assemble(buffer),
            // Descriptor::Desc4E(v) => v.assemble(buffer),
            Descriptor::Desc52(v) => v.assemble(buffer),
            Descriptor::Desc58(v) => v.assemble(buffer),
            Descriptor::Desc5A(v) => v.assemble(buffer),
            Descriptor::Desc83(v) => v.assemble(buffer),
            Descriptor::DescRaw(v) => buffer.extend_from_slice(&v),
        }
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
            // Descriptor::Desc48(v) => v.size(),
            // Descriptor::Desc4D(v) => v.size(),
            // Descriptor::Desc4E(v) => v.size(),
            Descriptor::Desc52(v) => v.size(),
            Descriptor::Desc58(v) => v.size(),
            Descriptor::Desc5A(v) => v.size(),
            Descriptor::Desc83(v) => v.size(),
            Descriptor::DescRaw(v) => v.len(),
        }
    }

    pub fn tag(&self) -> u8 {
        match self {
            Descriptor::Desc09(_) => 0x09,
            Descriptor::Desc0A(_) => 0x0A,
            Descriptor::Desc0E(_) => 0x0E,
            Descriptor::Desc40(_) => 0x40,
            Descriptor::Desc41(_) => 0x41,
            Descriptor::Desc43(_) => 0x43,
            Descriptor::Desc44(_) => 0x44,
            // Descriptor::Desc48(_) => 0x48,
            // Descriptor::Desc4D(_) => 0x4D,
            // Descriptor::Desc4E(_) => 0x4E,
            Descriptor::Desc52(_) => 0x52,
            Descriptor::Desc58(_) => 0x58,
            Descriptor::Desc5A(_) => 0x5A,
            Descriptor::Desc83(_) => 0x83,
            Descriptor::DescRaw(v) => v[0],
        }
    }
}


/// Array of descriptors
#[derive(Default, Clone)]
pub struct Descriptors(Vec<Descriptor>);


impl fmt::Debug for Descriptors {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt(f) }
}


impl BitWrap for Descriptors {
    fn pack(&self, _dst: &mut [u8]) -> Result<usize, BitWrapError> {
        unimplemented!();
    }

    fn unpack(&mut self, src: &[u8]) -> Result<usize, BitWrapError> {
        let mut skip: usize = 0;
        while src.len() >= skip + 2 {
            let next = skip + 2 + src[skip + 1] as usize;
            if next > src.len() {
                return Err(BitWrapError);
            }
            self.0.push(Descriptor::try_from(&src[skip .. next])?);
            skip = next;
        }
        Ok(skip)
    }
}


impl Descriptors {
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
