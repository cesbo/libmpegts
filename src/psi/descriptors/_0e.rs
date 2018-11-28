use base;


/// Maximum bitrate descriptor.
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
            bitrate: base::get_u22(&slice[2 ..])
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0E);
        buffer.push((Self::min_size() - 2) as u8);

        let skip = buffer.len();
        buffer.push(0xC0);  // reserved bits
        buffer.resize(skip + 3, 0x00);
        base::set_u22(&mut buffer[skip ..], self.bitrate);
    }
}
