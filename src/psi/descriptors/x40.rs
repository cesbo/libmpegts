// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 2;


/// The network name descriptor provides the network name in text form.
///
/// EN 300 468 - 6.2.27
#[derive(Debug, Default)]
pub struct Desc40 {
    /// Network name.
    pub name: StringDVB
}


impl Desc40 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            name: StringDVB::from(&slice[2 ..])
        }
    }
}


impl Desc for Desc40 {
    #[inline]
    fn tag(&self) -> u8 {
        0x40
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.name.size()
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x40);
        self.name.assemble_sized(buffer);
    }
}
