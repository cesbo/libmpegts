// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::{
        Psi,
        PsiDemux,
    },
};


/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;


/// Maximum section length without CRC
// const PAT_SECTION_SIZE: usize = 1024 - 4;


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
    #[bits(8)]
    pub table_id: u8,

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(1, skip = 0)]
    #[bits(2, skip = 0b11)]
    #[bits(12, into = self.set_section_length)]
    section_length: u16,

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
    #[bytes(self.section_length - 5 - 4)]
    pub items: Vec<PatItem>,
}


impl Default for Pat {
    fn default() -> Self {
        Pat {
            table_id: 0x00,
            section_syntax_indicator: 1,
            section_length: 0,
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
    fn set_section_length(&self, _value: u16) -> u16 {
        self.items.len() as u16 * 4 + 5 + 4
    }

    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 8 + 4 &&
        psi.buffer[0] == 0x00 &&
        psi.check()
    }

    /// Reads PSI packet and append data into the `Pat`
    pub fn parse(&mut self, psi: &Psi) {
        if self.check(&psi) {
            self.unpack(&psi.buffer).unwrap();
        }
    }
}


impl PsiDemux for Pat {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::default();
        let size = 8 + self.items.len() * 4;
        psi.buffer.resize(size, 0);
        self.pack(&mut psi.buffer).unwrap();
        vec![psi]
    }
}


impl From<&Psi> for Pat {
    fn from(psi: &Psi) -> Self {
        let mut pat = Pat::default();
        pat.parse(psi);
        pat
    }
}
