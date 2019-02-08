use crate::textcode::StringDVB;

/// The network name descriptor provides the network name in text form.
///
/// EN 300 468 - 6.2.27
#[derive(Debug, Default)]
pub struct Desc40 {
    /// Network name.
    pub name: StringDVB
}

impl Desc40 {
    #[inline]
    pub fn min_size() -> usize {
        2
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            name: StringDVB::from(&slice[2 ..])
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size() + self.name.size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x40);
        self.name.assemble_sized(buffer);
    }
}
