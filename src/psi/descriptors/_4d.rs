use textcode::StringDVB;

/// short_event_descriptor - provides the name of the event and a short
/// description of the event.
#[derive(Debug, Default)]
pub struct Desc4D {
    pub lang: StringDVB,
    pub name: StringDVB,
    pub text: StringDVB,
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

        out.lang.decode(&slice[2 .. 5]);

        if slice.len() == text_e {
            if name_e > name_s {
                out.name.decode(&slice[name_s .. name_e]);
            }
            if text_e > text_s {
                out.text.decode(&slice[text_s .. text_e]);
            }
        }

        out
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();

        buffer.push(0x4D);
        buffer.push(0x00);

        self.lang.encode(buffer);

        let s = buffer.len();
        buffer.push(0x00);
        self.name.encode(buffer);
        buffer[s] = (buffer.len() - s - 1) as u8;

        let s = buffer.len();
        buffer.push(0x00);
        self.text.encode(buffer);
        buffer[s] = (buffer.len() - s - 1) as u8;

        let size = buffer.len() - skip - 2;
        if size > 0xFF || self.lang.len() != 3 {
            buffer.resize(skip, 0x00);
        } else {
            buffer[skip + 1] = size as u8;
        }
    }
}
