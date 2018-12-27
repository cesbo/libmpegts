use crate::base;

/// Maximum bitrate descriptor.
///
/// ISO 13818-1 - 2.6.26
#[derive(Debug, Default)]
pub struct Desc0E {
    /// The value indicates an upper bound of the bitrate,
    /// including transport overhead, that will be encountered
    /// in this program element or program.
    pub bitrate: u32
}

impl Desc0E {
    #[inline]
    pub fn min_size() -> usize {
        5
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() == Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            bitrate: base::get_u24(&slice[2 ..]) & 0x003f_ffff,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0E);
        buffer.push((self.size() - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 3, 0x00);
        base::set_u24(&mut buffer[skip ..], 0xC0_0000 | self.bitrate);
    }
}
