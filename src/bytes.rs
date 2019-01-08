/// Bytes operations with array
pub trait Bytes {
    fn get_u16(&self) -> u16;
    fn set_u16(&mut self, value: u16);
    fn get_u24(&self) -> u32;
    fn set_u24(&mut self, value: u32);
    fn get_u32(&self) -> u32;
    fn set_u32(&mut self, value: u32);
}

impl Bytes for [u8] {
    #[inline]
    fn get_u16(&self) -> u16 {
        debug_assert!(self.len() >= 2);
        (u16::from(self[0]) << 8) | u16::from(self[1])
    }

    #[inline]
    fn set_u16(&mut self, value: u16) {
        debug_assert!(self.len() >= 2);
        self[0] = (value >> 8) as u8;
        self[1] = (value) as u8;
    }

    #[inline]
    fn get_u24(&self) -> u32 {
        debug_assert!(self.len() >= 3);
        (u32::from(self[0]) << 16) | (u32::from(self[1]) << 8) | u32::from(self[2])
    }

    #[inline]
    fn set_u24(&mut self, value: u32) {
        debug_assert!(self.len() >= 3);
        self[0] = (value >> 16) as u8;
        self[1] = (value >> 8) as u8;
        self[2] = (value) as u8;
    }

    #[inline]
    fn get_u32(&self) -> u32 {
        debug_assert!(self.len() >= 4);
        (u32::from(self[0]) << 24) | (u32::from(self[1]) << 16) | (u32::from(self[2]) << 8) | u32::from(self[3])
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
