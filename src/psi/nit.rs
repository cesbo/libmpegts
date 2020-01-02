// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::Descriptor,
};


pub const NIT_PID: u16 = 0x0010;


/// NIT Item.
#[derive(Debug, Default, BitWrap)]
pub struct NitItem {
    /// Identifier which serves as a label for identification of this
    /// TS from any other multiplex within the delivery system.
    #[bits(16)]
    pub tsid: u16,

    /// Label identifying the network_id of the originating delivery system.
    #[bits(16)]
    pub onid: u16,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = descriptors_length,
        value = self.descriptors_length())]

    /// List of descriptors.
    #[bytes(descriptors_length)]
    pub descriptors: Vec<Descriptor>,
}


impl NitItem {
    #[inline]
    fn descriptors_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn size(&self) -> usize { 6 + self.descriptors_length() }
}


/// The NIT conveys information relating to the physical organization
/// of the multiplexes/TSs carried via a given network,
/// and the characteristics of the network itself.
///
/// EN 300 468 - 5.2.1
#[derive(Debug, BitWrap)]
pub struct Nit {
    /// Identifies to which table the section belongs:
    /// * `0x40` - actual network
    /// * `0x41` - other network
    #[bits(8)]
    pub table_id: u8,

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(1, skip = 0b1)]
    #[bits(2, skip = 0b11)]
    #[bits(12,
        name = _section_length,
        value = self.size() - 3)]

    /// Identifier which serves as a label the delivery system,
    /// about which the NIT informs, from any other delivery system.
    #[bits(16)]
    pub network_id: u16,

    #[bits(2, skip = 0b11)]
    #[bits(5)]
    pub version: u8,

    #[bits(1)]
    current_next_indicator: u8,

    #[bits(8)]
    section_number: u8,

    #[bits(8)]
    last_section_number: u8,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = descriptors_length,
        value = self.descriptors_length())]

    /// List of descriptors.
    #[bytes(descriptors_length)]
    pub descriptors: Vec<Descriptor>,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = items_length,
        value = self.items_length())]

    /// List of NIT items.
    #[bytes(items_length)]
    pub items: Vec<NitItem>,
}


impl Default for Nit {
    fn default() -> Self {
        Nit {
            table_id: 0x40,
            section_syntax_indicator: 1,
            network_id: 0,
            version: 0,
            current_next_indicator: 1,
            section_number: 0,
            last_section_number: 0,
            descriptors: Vec::default(),
            items: Vec::default(),
        }
    }
}


impl Nit {
    #[inline]
    fn descriptors_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn items_length(&self) -> usize {
        self.items.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    pub (crate) fn size(&self) -> usize {
        12 +
        self.descriptors_length() +
        self.items_length() +
        4
    }
}
