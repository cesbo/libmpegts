// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;


/// PAT Item
#[derive(Debug, Default, BitWrap)]
pub struct PatItem {
    /// Program Number
    #[bits(16)]
    pub pnr: u16,

    /// TS Packet Idetifier
    #[bits(3, skip = 0b111)]
    #[bits(13)]
    pub pid: u16,
}


/// Program Association Table provides the correspondence between a `pnr` (Program Number) and
/// the `pid` value of the TS packets which carry the program definition.
#[derive(Debug, BitWrap)]
pub struct Pat {
    #[bits(8, skip = 0x00)]

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(1, skip = 0)]
    #[bits(2, skip = 0b11)]
    #[bits(12,
        name = section_length,
        value = self.size() - 3)]

    #[bits(16)]
    pub tsid: u16,

    #[bits(2, skip = 0b11)]
    #[bits(5)]
    pub version: u8,

    #[bits(1)]
    current_next_indicator: u8,

    #[bits(8)]
    section_number: u8,

    #[bits(8)]
    last_section_number: u8,

    /// List of the PAT Items
    #[bytes(section_length - 5 - 4)]
    pub items: Vec<PatItem>,
}


impl Default for Pat {
    #[inline]
    fn default() -> Self {
        Pat {
            section_syntax_indicator: 1,
            tsid: 0,
            version: 0,
            current_next_indicator: 1,
            section_number: 0,
            last_section_number: 0,
            items: Vec::default(),
        }
    }
}


impl Pat {
    #[inline]
    pub (crate) fn size(&self) -> usize {
        8 +
        self.items.len() * 4 +
        4
    }
}
