// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 8;


/// extended_event_descriptor - provides a detailed text description of
/// an event, which may be used in addition to the short event descriptor.
/// More than one extended event descriptor can be associated to allow
/// information about one event greater in length than 256 bytes to be
/// conveyed (number and last_number fields).
/// Text information can be structured into two columns, one giving
/// an item description field and the other the item text (items field).
///
/// Example items:
/// - desc: "Directors", text: "Anthony Russo, Joe Russo"
/// - desc: "Writers", text: "Christopher Markus, Stephen McFeely"
///
/// EN 300 468 - 6.2.15
#[derive(Debug)]
pub struct Desc4E {
    pub number: u8,
    pub last_number: u8,
    pub lang: StringDVB,
    pub items: Vec<(StringDVB, StringDVB)>,
    pub text: StringDVB,
}


impl Desc4E {
    pub fn check(slice: &[u8]) -> bool {
        if slice.len() < MIN_SIZE {
            return false;
        }

        let length_of_items = usize::from(slice[6]);
        let text_length = usize::from(slice[7 + length_of_items]);
        usize::from(slice[1]) == MIN_SIZE - 2 + length_of_items + text_length
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut items_s = 7;
        let items_e = items_s + slice[6] as usize;
        let text_s = items_e + 1;
        let text_e = text_s + slice[items_e] as usize;

        Desc4E {
            number: slice[2] >> 4,
            last_number: slice[2] & 0x0F,
            lang: StringDVB::from(&slice[3 .. 6]),
            items: {
                let mut out: Vec<(StringDVB, StringDVB)> = Vec::new();
                while items_s < items_e {
                    let item_desc_s = items_s + 1;
                    let item_desc_e = item_desc_s + slice[items_s] as usize;
                    let item_text_s = item_desc_e + 1;
                    let item_text_e = item_text_s + slice[item_desc_e] as usize;

                    let item_desc = StringDVB::from(&slice[item_desc_s .. item_desc_e]);
                    let item_text = StringDVB::from(&slice[item_text_s .. item_text_e]);

                    out.push((item_desc, item_text));
                    items_s = item_text_e;
                }
                out
            },
            text: StringDVB::from(&slice[text_s .. text_e]),
        }
    }
}


impl Desc for Desc4E {
    #[inline]
    fn tag(&self) -> u8 {
        0x4E
    }

    #[inline]
    fn size(&self) -> usize {
        let mut items_size = 0;
        for (item_desc, item_text) in &self.items {
            items_size += item_desc.size() + item_text.size();
        }
        MIN_SIZE + items_size + self.text.size()
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size() - 2;
        if size > 0xFF {
            return;
        }

        buffer.push(0x4E);
        buffer.push(size as u8);
        buffer.push(set_bits!(8, self.number, 4, self.last_number, 4));

        self.lang.assemble(buffer);

        {
            let skip = buffer.len();
            buffer.push(0x00);
            for (item_desc, item_text) in &self.items {
                item_desc.assemble_sized(buffer);
                item_text.assemble_sized(buffer);
            }
            buffer[skip] = (buffer.len() - skip - 1) as u8;
        }

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
            Desc4E,
        },
    };

    static DATA_4E: &[u8] = &[
        0x4e, 0x20, 0x00, 0x72, 0x75, 0x73, 0x00, 0x1a, 0x01, 0xb7, 0xd8, 0xdc, 0xd0, 0x20,
        0xd1, 0xeb, 0xe1, 0xe2, 0xe0, 0xde, 0x20, 0xdf, 0xe0, 0xd8, 0xd1, 0xdb, 0xd8, 0xd6, 0xd0, 0xd5,
        0xe2, 0xe1, 0xef, 0x2e];

    #[test]
    fn test_4e_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_4E);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc4E>();
        assert_eq!(desc.size(), DATA_4E.len());
        assert_eq!(desc.number, 0);
        assert_eq!(desc.last_number, 0);
        assert_eq!(desc.lang, textcode::StringDVB::from_str("rus", textcode::ISO6937));
        assert_eq!(desc.text, textcode::StringDVB::from_str("Зима быстро приближается.", textcode::ISO8859_5));
    }

    #[test]
    fn test_4e_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc4E {
            number: 0,
            last_number: 0,
            lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
            items: Vec::new(),
            text: textcode::StringDVB::from_str("Зима быстро приближается.", textcode::ISO8859_5),
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_4E);
    }
}
