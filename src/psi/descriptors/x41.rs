// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::bytes::*;
use super::Desc;


const MIN_SIZE: usize = 2;


#[derive(Debug)]
pub struct Desc41i {
    pub service_id: u16,
    pub service_type: u8,
}


/// Service List Descriptor - provides a means of listing the services by
/// service_id and service type
///
/// EN 300 468 - 6.2.35
#[derive(Debug, Default)]
pub struct Desc41 {
    /// List of pairs service_id (pnr) and service_type
    pub items: Vec<Desc41i>,
}


impl Desc41 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE &&
        ((slice.len() - 2) % 3) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;
        while slice.len() > skip {
            let service_id = slice[skip ..].get_u16();
            let service_type = slice[skip + 2];
            result.items.push(Desc41i {
                service_id,
                service_type,
            });
            skip += 3;
        }
        result
    }
}


impl Desc for Desc41 {
    #[inline]
    fn tag(&self) -> u8 {
        0x41
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.items.len() * 3
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x41;
        buffer[skip + 1] = (size - 2) as u8;
        skip += 2;

        for item in &self.items {
            buffer[skip ..].set_u16(item.service_id);
            buffer[skip + 2] = item.service_type;
            skip += 3;
        }
    }
}
