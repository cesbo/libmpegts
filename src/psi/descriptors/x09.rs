// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::bytes::*;
use super::Desc;


const MIN_SIZE: usize = 6;


/// The conditional access descriptor is used to specify both system-wide
/// conditional access management information such as EMMs and
/// elementary stream-specific information such as ECMs.
///
/// ISO 13818-1 - 2.6.16
#[derive(Debug, Default)]
pub struct Desc09 {
    /// Type of CA system.
    pub caid: u16,
    /// PID of the Transport Stream packets which shall contain
    /// either ECM or EMM information for the CA systems.
    pub pid: u16,
    /// Private data bytes.
    pub data: Vec<u8>
}


impl Desc09 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            caid: slice[2 ..].get_u16(),
            pid: slice[4 ..].get_u16() & 0x1FFF,
            data: Vec::from(&slice[6 ..]),
        }
    }
}


impl Desc for Desc09 {
    #[inline]
    fn tag(&self) -> u8 {
        0x09
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.data.len()
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 6, 0x00);
        buffer[skip] = 0x09;
        buffer[skip + 1] = (self.size() - 2) as u8;
        buffer[skip + 2 ..].set_u16(self.caid);
        buffer[skip + 4 ..].set_u16(0xE000 | self.pid);
        buffer.extend_from_slice(&self.data.as_slice());
    }
}
