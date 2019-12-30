// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use std::{
    fmt,
};

use bitwrap::{
    BitWrap,
    BitWrapError,
};

mod x09; pub use x09::*;
mod x0a; pub use x0a::*;
mod x0e; pub use x0e::*;
mod x41; pub use x41::*;
mod x43; pub use x43::*;
mod x44; pub use x44::*;
mod x52; pub use x52::*;
mod x58; pub use x58::*;
mod x5a; pub use x5a::*;
mod x83; pub use x83::*;


/// Descriptors extends the definitions of programs and program elements.
#[derive(Clone, Debug)]
pub enum Descriptor {
    None,
    Desc09(Desc09),
    Desc0A(Desc0A),
    Desc0E(Desc0E),
    Desc41(Desc41),
    Desc43(Desc43),
    Desc44(Desc44),
    Desc52(Desc52),
    Desc58(Desc58),
    Desc5A(Desc5A),
    Desc83(Desc83),
    DescRaw(Vec<u8>),
}


impl Default for Descriptor {
    #[inline]
    fn default() -> Self { Descriptor::None }
}


impl BitWrap for Descriptor {
    fn pack(&self, dst: &mut [u8]) -> Result<usize, BitWrapError> {
        match self {
            Descriptor::None => Ok(0),
            Descriptor::Desc09(v) => v.pack(dst),
            Descriptor::Desc0A(v) => v.pack(dst),
            Descriptor::Desc0E(v) => v.pack(dst),
            Descriptor::Desc41(v) => v.pack(dst),
            Descriptor::Desc43(v) => v.pack(dst),
            Descriptor::Desc44(v) => v.pack(dst),
            Descriptor::Desc52(v) => v.pack(dst),
            Descriptor::Desc58(v) => v.pack(dst),
            Descriptor::Desc5A(v) => v.pack(dst),
            Descriptor::Desc83(v) => v.pack(dst),
            Descriptor::DescRaw(v) => v.pack(dst),
        }
    }

    fn unpack(&mut self, src: &[u8]) -> Result<usize, BitWrapError> {
        if src.len() < 2 {
            return Err(BitWrapError);
        }

        let end = 2 + src[1] as usize;
        if src.len() < end {
            return Err(BitWrapError);
        }

        match src[0] {
            0x09 => {
                let mut desc = Desc09::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc09(desc);
                Ok(result)
            }

            0x0A => {
                let mut desc = Desc0A::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc0A(desc);
                Ok(result)
            }

            0x0E => {
                let mut desc = Desc0E::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc0E(desc);
                Ok(result)
            }

            0x41 => {
                let mut desc = Desc41::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc41(desc);
                Ok(result)
            }

            0x43 => {
                let mut desc = Desc43::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc43(desc);
                Ok(result)
            }

            0x44 => {
                let mut desc = Desc44::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc44(desc);
                Ok(result)
            }

            0x52 => {
                let mut desc = Desc52::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc52(desc);
                Ok(result)
            }

            0x58 => {
                let mut desc = Desc58::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc58(desc);
                Ok(result)
            }

            0x5A => {
                let mut desc = Desc5A::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc5A(desc);
                Ok(result)
            }

            0x83 => {
                let mut desc = Desc83::default();
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::Desc83(desc);
                Ok(result)
            }

            _ => {
                let mut desc: Vec<u8> = Vec::with_capacity(end);
                let result = desc.unpack(&src[.. end])?;
                *self = Descriptor::DescRaw(desc);
                Ok(result)
            }
        }
    }
}


impl Descriptor {
    pub (crate) fn size(&self) -> usize {
        match self {
            Descriptor::None => 0,
            Descriptor::Desc09(v) => v.size(),
            Descriptor::Desc0A(v) => v.size(),
            Descriptor::Desc0E(v) => v.size(),
            Descriptor::Desc41(v) => v.size(),
            Descriptor::Desc43(v) => v.size(),
            Descriptor::Desc44(v) => v.size(),
            Descriptor::Desc52(v) => v.size(),
            Descriptor::Desc58(v) => v.size(),
            Descriptor::Desc5A(v) => v.size(),
            Descriptor::Desc83(v) => v.size(),
            Descriptor::DescRaw(v) => v.len(),
        }
    }
}
