use textcode;

/// short_event_descriptor - provides the name of the event and a short
/// description of the event.
#[derive(Debug, Default)]
pub struct Desc4D {
    /// Language
    pub lang: String,
    /// Event name (title)
    pub name: String,
    /// Event short description (sub-title)
    pub text: String,
    /// Code page for name and text fields
    pub codepage: usize,
}

impl Desc4D {
    pub fn check(ptr: &[u8]) -> bool {
        ptr[1] >= 5
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut out = Desc4D::default();

        let name_s = 6;
        let name_e = name_s + slice[5] as usize;
        let text_s = name_e + 1;
        let text_e = text_s + slice[name_e] as usize;

        textcode::decode(&mut out.lang, &slice[2 .. 5]);

        if slice.len() == text_e {
            out.codepage = textcode::decode(&mut out.name, &slice[name_s .. name_e]);

            if text_e > text_s {
                textcode::decode(&mut out.text, &slice[text_s .. text_e]);
            }
        }

        out
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();

        buffer.push(0x4D);
        buffer.push(0x00);

        textcode::encode(&self.lang, buffer, 0);

        let s = buffer.len();
        buffer.push(0x00);
        textcode::encode(&self.name, buffer, self.codepage);
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
