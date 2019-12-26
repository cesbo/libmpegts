// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 7;


/// short_event_descriptor - provides the name of the event and a short
/// description of the event.
///
/// EN 300 468 - 6.2.37
#[derive(Debug, Default, Clone)]
pub struct Desc4D {
    /// Language
    pub lang: StringDVB,
    /// Event name (title)
    pub name: StringDVB,
    /// Event short description (sub-title)
    pub text: StringDVB,
}


impl Desc4D {
    pub fn check(slice: &[u8]) -> bool {
        if slice.len() < MIN_SIZE {
            return false;
        }

        let event_name_length = usize::from(slice[5]);
        let text_length = usize::from(slice[6 + event_name_length]);
        usize::from(slice[1]) == MIN_SIZE - 2 + event_name_length + text_length
    }

    pub fn parse(slice: &[u8]) -> Self {
        let name_s = 6;
        let name_e = name_s + slice[5] as usize;
        let text_s = name_e + 1;
        let text_e = text_s + slice[name_e] as usize;

        Desc4D {
            lang: StringDVB::from(&slice[2 .. 5]),
            name: StringDVB::from(&slice[name_s .. name_e]),
            text: StringDVB::from(&slice[text_s .. text_e]),
        }
    }
}


impl Desc for Desc4D {
    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.name.size() + self.text.size()
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x4D);
        buffer.push((self.size() - 2) as u8);

        self.lang.assemble(buffer);
        self.name.assemble_sized(buffer);
        self.text.assemble_sized(buffer);
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        textcode,
        psi::{
            Desc,
            Descriptors,
            Desc4D,
        },
    };

    static DATA_4D: &[u8] = &[
        0x4d, 0x18, 0x72, 0x75, 0x73, 0x13, 0x01, 0xc1, 0xe2, 0xe0, 0xde, 0xd9, 0xda, 0xd0, 0x20, 0xdd,
        0xd0, 0x20, 0xb0, 0xdb, 0xef, 0xe1, 0xda, 0xd5, 0x2e, 0x00];

    #[test]
    fn test_4d_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_4D);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc4D>();
        assert_eq!(desc.size(), DATA_4D.len());
        assert_eq!(desc.lang, textcode::StringDVB::from_str("rus", textcode::ISO6937));
        assert_eq!(desc.name, textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5));
        assert!(desc.text.is_empty());
    }

    #[test]
    fn test_4d_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc4D {
            lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
            name: textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5),
            text: textcode::StringDVB::default(),
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_4D);
    }
}
