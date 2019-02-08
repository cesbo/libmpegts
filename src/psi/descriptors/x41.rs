use crate::bytes::*;

/// Service List Descriptor - provides a means of listing the services by
/// service_id and service type
///
/// EN 300 468 - 6.2.35
#[derive(Debug, Default)]
pub struct Desc41 {
    /// List of pairs service_id (pnr) and service_type
    pub items: Vec<(u16, u8)>,
}

impl Desc41 {
    #[inline]
    pub fn min_size() -> usize {
        2
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size() &&
        ((slice.len() - 2) % 3) == 0
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        let mut skip = 2;
        while slice.len() > skip {
            let service_id = slice[skip ..].get_u16();
            let service_type = slice[skip + 2];
            result.items.push((service_id, service_type));
            skip += 3;
        }
        result
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size() + self.items.len() * 3
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let mut skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x41;
        buffer[skip + 1] = (size - 2) as u8;
        skip += 2;

        for item in &self.items {
            buffer[skip ..].set_u16(item.0);
            buffer[skip + 2] = item.1;
            skip += 3;
        }
    }
}
