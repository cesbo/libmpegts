use crate::textcode::StringDVB;

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
    #[inline]
    pub fn min_size() -> usize {
        8
    }

    pub fn check(slice: &[u8]) -> bool {
        if slice.len() < Self::min_size() {
            return false;
        }

        let length_of_items = usize::from(slice[6]);
        let text_length = usize::from(slice[7 + length_of_items]);
        usize::from(slice[1]) == Self::min_size() - 2 + length_of_items + text_length
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

    #[inline]
    pub fn size(&self) -> usize {
        let mut items_size = 0;
        for (item_desc, item_text) in &self.items {
            items_size += item_desc.size() + item_text.size();
        }
        Self::min_size() + items_size + self.text.size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
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