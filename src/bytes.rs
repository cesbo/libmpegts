/// Bytes operations with array
pub trait Bytes {
    fn set_u16(&mut self, value: u16);
    fn set_u24(&mut self, value: u32);
    fn set_u32(&mut self, value: u32);
}

impl Bytes for [u8] {
    #[inline]
    fn set_u16(&mut self, value: u16) {
        debug_assert!(self.len() >= 2);
        self[0] = (value >> 8) as u8;
        self[1] = (value) as u8;
    }

    #[inline]
    fn set_u24(&mut self, value: u32) {
        debug_assert!(self.len() >= 3);
        self[0] = (value >> 16) as u8;
        self[1] = (value >> 8) as u8;
        self[2] = (value) as u8;
    }

    #[inline]
    fn set_u32(&mut self, value: u32) {
        debug_assert!(self.len() >= 4);
        self[0] = (value >> 24) as u8;
        self[1] = (value >> 16) as u8;
        self[2] = (value >> 8) as u8;
        self[3] = (value) as u8;
    }
}

#[cfg(test)]
mod tests {
    use crate::bytes::*;

    #[test]
    fn test_set_bytes_u16() {
        let mut data = Vec::<u8>::new();
        data.resize(2, 0x00);
        data[0 ..].set_u16(0x1234);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
    }

    #[test]
    fn test_set_bytes_u24() {
        let mut data = Vec::<u8>::new();
        data.resize(3, 0x00);
        data[0 ..].set_u24(0x1234AB);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0xAB);
    }

    #[test]
    fn test_set_bytes_u32() {
        let mut data = Vec::<u8>::new();
        data.resize(4, 0x00);
        data[0 ..].set_u32(0x1234ABCD);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0xAB);
        assert_eq!(data[3], 0xCD);
    }
}
