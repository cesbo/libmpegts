use crate::bytes::*;

/// Logical Channel Descriptor - provides a default channel number label for service
///
/// HD-BOOK-DTT - 7.3.1
#[derive(Debug, Default)]
pub struct Desc83 {
    /// List of pairs service_id (pnr), visible flag, and channel number
    pub items: Vec<(u16, u8, u16)>,
}

impl Desc83 {
    #[inline]
    pub fn min_size() -> usize {
        2
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size() &&
        ((slice.len() - 2) % 4) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;
        while slice.len() >= skip + 4 {
            let pnr = slice[skip ..].get_u16();
            let visible = slice[skip + 2] >> 7;
            let lcn = slice[skip + 2 ..].get_u16() & 0x03FF;
            result.items.push((pnr, visible, lcn));
            skip += 4;
        }
        result
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size() + self.items.len() * 4
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x83;
        skip += 1;
        buffer[skip] = (size - 2) as u8;
        skip += 1;

        for item in &self.items {
            buffer[skip ..].set_u16(item.0);
            buffer[skip + 2 ..].set_u16((u16::from(item.1) << 15) | 0x7C00 | item.2);
            skip += 4;
        }
    }
}
