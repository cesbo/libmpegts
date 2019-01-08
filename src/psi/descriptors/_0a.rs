use crate::textcode::StringDVB;

#[allow(non_camel_case_types)]
#[derive(Debug, Default)]
pub struct Desc0A_Item {
    /// Identifies the language or languages used by the associated program element.
    pub code: StringDVB,
    /// Type of audio stream.
    pub audio_type: u8
}

impl Desc0A_Item {
    pub fn parse(slice: &[u8]) -> Self {
        Self {
            code: StringDVB::from(&slice[0 .. 3]),
            audio_type: slice[3]
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        self.code.assemble(buffer);
        buffer.push(self.audio_type);
    }
}


/// The language descriptor is used to specify the language
/// of the associated program element.
///
/// ISO 13818-1 - 2.6.18
#[derive(Debug, Default)]
pub struct Desc0A {
    pub items: Vec<Desc0A_Item>
}

impl Desc0A {
    #[inline]
    pub fn min_size() -> usize {
        2
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();

        let ptr = &slice[2 .. 2 + slice[1] as usize];
        let item_len = 4;

        let mut skip = 0;
        loop {
            let end = skip + item_len;
            if end > ptr.len() {
                break;
            }
            result.items.push(Desc0A_Item::parse(&ptr[skip .. end]));
            skip = end;
        }

        result
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size() + self.items.len() * 4
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0A);
        buffer.push((self.size() - 2) as u8);

        for item in &self.items {
            item.assemble(buffer);
        }
    }
}