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
    #[inline]
    pub fn min_size() -> usize {
        5
    }

    pub fn check(slice: &[u8]) -> bool {
        if slice.len() < Self::min_size() + 2 {
            return false;
        }

        let event_name_length = usize::from(slice[5]);
        let text_length = usize::from(slice[6 + event_name_length]);
        usize::from(slice[1]) == Self::min_size() + event_name_length + text_length
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
        buffer.push(0x4D);

        let skip = buffer.len();
        buffer.push(0x00);

        self.lang.assemble(buffer);
        self.name.assemble_sized(buffer);
        self.text.assemble_sized(buffer);

        let size = buffer.len() - skip - 1;
        buffer[skip] = size as u8;
    }
}
