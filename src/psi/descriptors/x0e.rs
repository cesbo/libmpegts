use crate::bytes::*;


const MIN_SIZE: usize = 5;


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
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            bitrate: slice[2 ..].get_u24() & 0x003F_FFFF,
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        MIN_SIZE
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x0E;
        buffer[skip + 1] = (size - 2) as u8;
        buffer[skip + 2 ..].set_u24(0x00C0_0000 | self.bitrate);
    }
}
