use textcode;

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
#[derive(Debug)]
pub struct Desc4E {
    pub number: u8,
    pub last_number: u8,
    pub lang: String,
    pub items: Vec<(String, String)>,
    pub text: String,
    pub codepage: usize,
}

impl Desc4E {
    pub fn check(ptr: &[u8]) -> bool {
        ptr[1] >= 6
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut items_s = 7;
        let items_e = items_s + slice[6] as usize;
        let text_s = items_e + 1;
        let text_e = text_s + slice[items_e] as usize;

        let mut out = Desc4E {
            number: slice[2] >> 4,
            last_number: slice[2] & 0x0F,
            lang: String::new(),
            items: Vec::new(),
            text: String::new(),
            codepage: 0,
        };

        textcode::decode(&mut out.lang, &slice[3 .. 6]);

        while items_s < items_e {
            let item_desc_s = items_s + 1;
            let item_desc_e = item_desc_s + slice[items_s] as usize;
            let item_text_s = item_desc_e + 1;
            let item_text_e = item_text_s + slice[item_desc_e] as usize;

            let mut item_desc = String::new();
            textcode::decode(&mut item_desc, &slice[item_desc_s .. item_desc_e]);

            let mut item_text = String::new();
            out.codepage = textcode::decode(&mut item_text, &slice[item_text_s .. item_text_e]);

            out.items.push((item_desc, item_text));
            items_s = item_text_e;
        }

        if text_e > text_s {
            out.codepage = textcode::decode(& mut out.text, &slice[text_s .. text_e]);
        }

        out
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();

        buffer.push(0x4E);
        buffer.push(0x00);
        buffer.push(((self.number & 0x0F) << 4) | (self.last_number & 0x0F));

        textcode::encode(&self.lang, buffer, 0);

        let s = buffer.len();
        buffer.push(0x00);
        for (item_desc, item_text) in self.items.iter() {
            let s = buffer.len();
            buffer.push(0x00);
            textcode::encode(&item_desc, buffer, self.codepage);
            buffer[s] = (buffer.len() - s - 1) as u8;

            let s = buffer.len();
            buffer.push(0x00);
            textcode::encode(&item_text, buffer, self.codepage);
            buffer[s] = (buffer.len() - s - 1) as u8;
        }
        buffer[s] = (buffer.len() - s - 1) as u8;

        let s = buffer.len();
        buffer.push(0x00);
        textcode::encode(&self.text, buffer, self.codepage);
        buffer[s] = (buffer.len() - s - 1) as u8;

        let size = buffer.len() - skip - 2;
        if size > 0xFF || self.lang.len() != 3 {
            buffer.resize(skip, 0x00);
        } else {
            buffer[skip + 1] = size as u8;
        }
    }
}
