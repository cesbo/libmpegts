use crate::textcode::StringDVB;
use super::Desc;


const MIN_SIZE: usize = 2;


/// The language descriptor is used to specify the language
/// of the associated program element.
///
/// ISO 13818-1 - 2.6.18
#[derive(Debug, Default)]
pub struct Desc0A {
    /// 0 - Identifies the language or languages used by the associated program element
    /// 1 - Type of audio stream
    pub items: Vec<(StringDVB, u8)>
}


impl Desc0A {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= MIN_SIZE &&
        ((slice.len() - 2) % 4) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;

        while slice.len() > skip {
            let code = StringDVB::from(&slice[skip .. skip + 3]);
            let audio_type = slice[skip + 3];
            result.items.push((code, audio_type));
            skip += 4;
        }
        result
    }
}


impl Desc for Desc0A {
    #[inline]
    fn tag(&self) -> u8 {
        0x0A
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE + self.items.len() * 4
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0A);
        buffer.push((self.size() - 2) as u8);

        for item in &self.items {
            item.0.assemble(buffer);
            buffer.push(item.1);
        }
    }
}
