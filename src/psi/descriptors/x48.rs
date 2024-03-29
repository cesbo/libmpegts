// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 5;


/// The service descriptor provides the names of the service provider
/// and the service in text form together with the service_type.
///
/// EN 300 468 - 6.2.33
#[derive(Debug, Default, Clone)]
pub struct Desc48 {
    /// Type of the service.
    pub service_type: u8,
    /// Name of the service provider.
    pub provider: StringDVB,
    /// Name of the service.
    pub name: StringDVB,
}


impl Desc48 {
    pub fn check(slice: &[u8]) -> bool {
        if slice.len() < MIN_SIZE {
            return false;
        }

        let provider_length = usize::from(slice[3]);
        let name_length = usize::from(slice[4 + provider_length]);

        usize::from(slice[1]) == MIN_SIZE - 2 + provider_length + name_length
    }

    pub fn parse(slice: &[u8]) -> Self {
        let provider_s = 4;
        let provider_e = provider_s + usize::from(slice[3]);
        let name_s = provider_e + 1;
        let name_e = name_s + usize::from(slice[provider_e]);

        Self {
            service_type: slice[2],
            provider: StringDVB::from(&slice[provider_s .. provider_e]),
            name: StringDVB::from(&slice[name_s .. name_e]),
        }
    }
}


impl Desc for Desc48 {
    #[inline]
    fn tag(&self) -> u8 {
        0x48
    }

    fn size(&self) -> usize {
        MIN_SIZE + self.provider.size() + self.name.size()
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x48);
        buffer.push((self.size() - 2) as u8);

        buffer.push(self.service_type);

        self.provider.assemble_sized(buffer);
        self.name.assemble_sized(buffer);
    }
}
