// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::{
        Descriptor,
        utils::crc32b,
    },
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
        name = item_info_length,
        value = self.item_info_length())]

    /// List of descriptors.
    #[bytes(item_info_length)]
    pub descriptors: Vec<Descriptor>,
}


impl NitItem {
    #[inline]
    fn item_info_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn size(&self) -> usize { 6 + self.item_info_length() }
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
        name = section_length,
        value = self.size() - 3,
        min = 9 + 4,
        max = 1021)]

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
        name = info_length,
        value = self.info_length(),
        max = section_length - 9 - 4)]

    /// List of descriptors.
    #[bytes(info_length)]
    pub descriptors: Vec<Descriptor>,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = stream_loop_length,
        value = self.stream_loop_length(),
        max = section_length - 9 - 4 - info_length)]

    /// List of NIT items.
    #[bytes(stream_loop_length)]
    pub items: Vec<NitItem>,

    // TODO: if name not defined use field
    #[bits(32,
        name = _crc,
        value = crc32b(&dst[.. offset]),
        eq = crc32b(&src[.. offset]))]
    pub _crc: u32,
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
            _crc: 0,
        }
    }
}


impl Nit {
    #[inline]
    fn info_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn stream_loop_length(&self) -> usize {
        self.items.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    pub (crate) fn size(&self) -> usize {
        12 +
        self.info_length() +
        self.stream_loop_length() +
        4
    }
}
