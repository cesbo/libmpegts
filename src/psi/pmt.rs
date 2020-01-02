// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;

use crate::{
    psi::Descriptor,
    es::StreamType,
};


/// PMT Item
#[derive(Debug, Default, BitWrap)]
pub struct PmtItem {
    /// This field specifying the type of program element
    /// carried within the packets with the PID.
    #[bits(8)]
    pub stream_type: u8,

    /// This field specifying the PID of the Transport Stream packets
    /// which carry the associated program element.
    #[bits(3, skip = 0b111)]
    #[bits(13)]
    pub pid: u16,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = es_info_length,
        value = self.es_info_length())]

    /// List of descriptors.
    #[bytes(es_info_length)]
    pub descriptors: Vec<Descriptor>,
}


impl PmtItem {
    #[inline]
    fn es_info_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    fn size(&self) -> usize { 5 + self.es_info_length() }

    pub fn get_stream_type(&self) -> StreamType {
        match self.stream_type {
            // Video
            0x01 => StreamType::VIDEO,  // ISO/IEC 11172 Video
            0x02 => StreamType::VIDEO,  // ISO/IEC 13818-2 Video
            0x10 => StreamType::VIDEO,  // ISO/IEC 14496-2 Visual
            0x1B => StreamType::VIDEO,  // ISO/IEC 14496-10 Video | H.264
            0x24 => StreamType::VIDEO,  // ISO/IEC 23008-2 Video | H.265
            // Audio
            0x03 => StreamType::AUDIO,  // ISO/IEC 11172 Audio
            0x04 => StreamType::AUDIO,  // ISO/IEC 13818-3 Audio
            0x0F => StreamType::AUDIO,  // ISO/IEC 13818-7 Audio (ADTS)
            0x11 => StreamType::AUDIO,  // ISO/IEC 14496-3 Audio (LATM)
            // Private Data
            0x05 => {
                // for desc in self.descriptors.iter() {
                //     if desc.tag() == 0x6F {                 // application_signalling_descriptor
                //         return StreamType::AIT;
                //     }
                // }
                StreamType::DATA
            }
            0x06 => {
                // for desc in self.descriptors.iter() {
                //     match desc.tag() {
                //         0x56 => return StreamType::TTX,     // teletext_descriptor
                //         0x59 => return StreamType::SUB,     // subtitling_descriptor
                //         0x6A => return StreamType::AUDIO,   // AC-3_descriptor
                //         0x7A => return StreamType::AUDIO,   // enhanced_AC-3_descriptor
                //         0x81 => return StreamType::AUDIO,   // AC-3 Audio
                //         _ => {}
                //     }
                // }
                StreamType::DATA
            }
            _ => StreamType::DATA,
        }
    }
}


/// Program Map Table - provides the mappings between program numbers
/// and the program elements that comprise them.
#[derive(Debug, BitWrap)]
pub struct Pmt {
    #[bits(8, skip = 0x02)]

    #[bits(1)]
    pub section_syntax_indicator: u8,

    #[bits(1, skip = 0)]
    #[bits(2, skip = 0b11)]
    #[bits(12,
        name = section_length,
        value = self.size() - 3)]

    /// Program number
    #[bits(16)]
    pub pnr: u16,

    #[bits(2, skip = 0b11)]
    #[bits(5)]
    pub version: u8,

    #[bits(1)]
    current_next_indicator: u8,

    #[bits(8)]
    section_number: u8,

    #[bits(8)]
    last_section_number: u8,

    /// PCR (Program Clock Reference) pid.
    #[bits(3, skip = 0b111)]
    #[bits(13)]
    pub pcr: u16,

    #[bits(4, skip = 0b1111)]
    #[bits(12,
        name = info_length,
        value = self.info_length())]

    /// List of descriptors.
    #[bytes(info_length)]
    pub descriptors: Vec<Descriptor>,

    /// List of PMT items.
    /// TODO: check if (section_len + info_len) < 13
    #[bytes(section_length - info_length - 9 - 4)]
    pub items: Vec<PmtItem>,
}


impl Default for Pmt {
    #[inline]
    fn default() -> Self {
        Pmt {
            section_syntax_indicator: 1,
            pnr: 0,
            version: 0,
            current_next_indicator: 1,
            section_number: 0,
            last_section_number: 0,
            pcr: 0,
            descriptors: Vec::default(),
            items: Vec::default(),
        }
    }
}


impl Pmt {
    #[inline]
    fn info_length(&self) -> usize {
        self.descriptors.iter().fold(0, |acc, item| acc + item.size())
    }

    #[inline]
    pub (crate) fn size(&self) -> usize {
        12 +
        self.info_length() +
        self.items.iter().fold(0, |acc, item| acc + item.size()) +
        4
    }
}
