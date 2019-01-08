/// BCD (Binary-Coded Decimal) is a class of binary encodings of
/// decimal numbers where each decimal digit is represented
/// by a fixed number of bits.
pub trait BCD<T> {
    fn from_bcd(value: T) -> T;
    fn to_bcd(self) -> T;
}

impl BCD<u8> for u8 {
    #[inline]
    fn from_bcd(value: u8) -> u8 {
        (((value & 0xF0) >> 4) * 10) +
        (value & 0x0F)
    }

    #[inline]
    fn to_bcd(self) -> u8 {
        ((self / 10) << 4) +
        (self % 10)
    }
}

impl BCD<u16> for u16 {
    #[inline]
    fn from_bcd(value: u16) -> u16 {
        (u16::from(u8::from_bcd((value >> 8) as u8)) * 100) +
        u16::from(u8::from_bcd((value & 0xFF) as u8))
    }

    #[inline]
    fn to_bcd(self) -> u16 {
        (u16::from(u8::to_bcd((self / 100) as u8)) << 8) +
        u16::from(u8::to_bcd((self % 100) as u8))
    }
}

impl BCD<u32> for u32 {
    #[inline]
    fn from_bcd(value: u32) -> u32 {
        (u32::from(u16::from_bcd((value >> 16) as u16)) * 10000) +
        u32::from(u16::from_bcd((value & 0xFFFF) as u16))
    }

    #[inline]
    fn to_bcd(self) -> u32 {
        (u32::from(u16::to_bcd((self / 10000) as u16)) << 16) +
        u32::from(u16::to_bcd((self % 10000) as u16))
    }
}

/// Converts between Unix Timestamp and Binary Coded Decimal Time
pub trait BCDTime {
    fn from_bcd_time(value: u32) -> u32;
    fn to_bcd_time(self) -> u32;
}

impl BCDTime for u32 {
    #[inline]
    fn from_bcd_time(value: u32) -> u32 {
        (u32::from(u8::from_bcd((value >> 16) as u8)) * 3600) +
        (u32::from(u8::from_bcd((value >> 8) as u8)) * 60) +
        u32::from(u8::from_bcd(value as u8))
    }

    #[inline]
    fn to_bcd_time(self) -> u32 {
        (u32::from(u8::to_bcd((self / 3600 % 24) as u8)) << 16) +
        (u32::from(u8::to_bcd((self / 60 % 60) as u8)) << 8) +
        u32::from(u8::to_bcd((self % 60) as u8))
    }
}
