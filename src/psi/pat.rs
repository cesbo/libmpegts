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
const PAT_SECTION_SIZE: usize = 1024 - 4;


/// PAT Item
#[derive(Debug, Default, BitWrap)]
pub struct PatItem {
    /// Program Number
    #[bits(16)] pub pnr: u16,
    #[bits_skip(3, 0b111)]
    /// TS Packet Idetifier
    #[bits(13)] pub pid: u16,
}


/// Program Association Table provides the correspondence between a `pnr` (Program Number) and
/// the `pid` value of the TS packets which carry the program definition.
#[derive(Debug, BitWrap)]
pub struct Pat {
    #[bits(8)] pub table_id: u8,
    #[bits(1)] pub section_syntax_indicator: u8,
    #[bits_skip(1, 0)]
    #[bits_skip(2, 0b11)]
    #[bits(12)] section_length: u16,
    #[bits(16)] pub tsid: u16,
    #[bits_skip(2, 0b11)]
    #[bits(5)] pub version: u8,
    #[bits(1)] current_next_indicator: u8,
    #[bits(8)] section_number: u8,
    #[bits(8)] last_section_number: u8,

    /// List of the PAT Items
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
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 8 + 4 &&
        psi.buffer[0] == 0x00 &&
        psi.check()
    }

    /// Reads PSI packet and append data into the `Pat`
    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.unpack(&psi.buffer).unwrap();

        let ptr = &psi.buffer[8 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 4 {
            let mut item = PatItem::default();
            item.unpack(&ptr[skip ..]).unwrap();
            self.items.push(item);
            skip += 4;
        }
    }
}


impl PsiDemux for Pat {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::default();
        let mut skip = 0;
        psi.buffer.resize(psi.buffer.len() + 8, 0);
        skip += self.pack(&mut psi.buffer[skip ..]).unwrap();

        for item in &self.items {
            if psi.buffer.len() + 4 > PAT_SECTION_SIZE {
                break;
            }
            psi.buffer.resize(psi.buffer.len() + 4, 0);
            skip += item.pack(&mut psi.buffer[skip ..]).unwrap();
        }

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
