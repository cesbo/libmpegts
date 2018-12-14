pub trait BCD<T> {
    fn from_bcd(value: T) -> T;
    fn to_bcd(&self) -> T;
}

impl BCD<u8> for u8 {
    fn from_bcd(value: u8) -> u8 {
        ((value & 0xF0) >> 4) * 10 + (value & 0x0F)
    }

    fn to_bcd(&self) -> u8 {
        (((self / 10) << 4) | (self % 10)) as u8
    }
}

impl BCD<u16> for u16 {
    fn from_bcd(value: u16) -> u16 {
        (u8::from_bcd((value >> 8) as u8) as u16) * 100 +
            (u8::from_bcd((value & 0xFF) as u8) as u16)
    }

    fn to_bcd(&self) -> u16 {
        ((u8::to_bcd(&((self / 100) as u8)) as u16) << 8) +
            (u8::to_bcd(&((self % 100) as u8)) as u16)
    }
}

impl BCD<u32> for u32 {
    fn from_bcd(value: u32) -> u32 {
        (u16::from_bcd((value >> 16) as u16) as u32) * 10000 +
            (u16::from_bcd((value & 0xFFFF) as u16) as u32)
    }

    fn to_bcd(&self) -> u32 {
        ((u16::to_bcd(&((self / 10000) as u16)) as u32) << 16) +
            (u16::to_bcd(&((self % 10000) as u16)) as u32)
    }
}
