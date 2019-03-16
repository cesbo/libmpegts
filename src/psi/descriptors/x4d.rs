use crate::textcode::StringDVB;


const MIN_SIZE: usize = 7;


/// short_event_descriptor - provides the name of the event and a short
/// description of the event.
///
/// EN 300 468 - 6.2.37
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

    #[inline]
    pub fn size(&self) -> usize {
        MIN_SIZE + self.name.size() + self.text.size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x4D);
        buffer.push((self.size() - 2) as u8);

        self.lang.assemble(buffer);
        self.name.assemble_sized(buffer);
        self.text.assemble_sized(buffer);
    }
}
