/// Bytes operations with array
pub trait Bytes {
    /// Gets 16 bits unsigned integer from byte array
    fn get_u16(&self) -> u16;
    /// Sets 16 bits unsigned integer to byte array
    fn set_u16(&mut self, value: u16);
    /// Gets 24 bits unsigned integer from byte array
    fn get_u24(&self) -> u32;
    /// Sets 24 bits unsigned integer to byte array
    fn set_u24(&mut self, value: u32);
    /// Gets 32 bits unsigned integer from byte array
    fn get_u32(&self) -> u32;
    /// Sets 32 bits unsigned integer to byte array
    fn set_u32(&mut self, value: u32);
    /// Gets PID (13 bits unsigned integer) from byte array
    fn get_pid(&self) -> u16;
    /// Sets PID (13 bits unsigned integer) to byte array
    /// Sets first 3 bits as a reserved (0xE0) in the first byte
    fn set_pid(&mut self, value: u16);
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

    #[inline]
    fn get_pid(&self) -> u16 {
        self.get_u16() & 0x1FFF
    }

    #[inline]
    fn set_pid(&mut self, value: u16) {
        self.set_u16(0xE000 | value);
    }
}
