use textcode::*;

/// short_event_descriptor - provides the name of the event and a short
/// description of the event.
#[derive(Debug, Default)]
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
        if slice[1] < 7 {
            return false;
        }

        let s = 6 + slice[5] as usize;
        if slice.len() < s {
            return false;
        }

        let s = s + 1 + slice[s] as usize;
        slice.len() == s
    }

    pub fn parse(slice: &[u8]) -> Self {
        let name_s = 6;
        let name_e = name_s + slice[5] as usize;
        let text_s = name_e + 1;
        let text_e = text_s + slice[name_e] as usize;

        Desc4D {
            lang: StringDVB::from_raw(&slice[2 .. 5]),
            name: StringDVB::from_raw(&slice[name_s .. name_e]),
            text: StringDVB::from_raw(&slice[text_s .. text_e]),
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();

        buffer.push(0x4D);
        buffer.push(0x00);

        self.lang.assemble(buffer, false);
        self.name.assemble(buffer, true);
        self.text.assemble(buffer, true);

        let size = buffer.len() - skip - 2;
        if size > 0xFF {
            buffer.resize(skip, 0x00);
        } else {
            buffer[skip + 1] = size as u8;
        }
    }
}
