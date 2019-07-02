/// BCD (Binary-Coded Decimal) is a class of binary encodings of
/// decimal numbers where each decimal digit is represented
/// by a fixed number of bits.
pub trait BCD {
    fn from_bcd(self) -> Self;
    fn to_bcd(self) -> Self;
}


impl BCD for u8 {
    #[inline]
    fn from_bcd(self) -> Self {
        debug_assert!((self & 0xF0) < 0xA0);
        debug_assert!((self & 0x0F) < 0x0A);
        self - (self >> 4) * 6
    }

    #[inline]
    fn to_bcd(self) -> Self {
        debug_assert!(self < 100);
        self + (self / 10) * 6
    }
}


impl BCD for u16 {
    #[inline]
    fn from_bcd(self) -> Self {
        (u16::from(u8::from_bcd((self >> 8) as u8)) * 100) + u16::from(u8::from_bcd(self as u8))
    }

    #[inline]
    fn to_bcd(self) -> Self {
        (u16::from(u8::to_bcd((self / 100) as u8)) << 8) + u16::from(u8::to_bcd((self % 100) as u8))
    }
}


impl BCD for u32 {
    #[inline]
    fn from_bcd(self) -> Self {
        (u32::from(u16::from_bcd((self >> 16) as u16)) * 10000) + u32::from(u16::from_bcd(self as u16))
    }

    #[inline]
    fn to_bcd(self) -> Self {
        (u32::from(u16::to_bcd((self / 10000) as u16)) << 16) + u32::from(u16::to_bcd((self % 10000) as u16))
    }
}


/// Converts between Unix Timestamp and Binary Coded Decimal Time
pub trait BCDTime {
    fn from_bcd_time(self) -> Self;
    fn to_bcd_time(self) -> Self;
}


/// Converts u16 bcd to minutes
impl BCDTime for u16 {
    #[inline]
    fn from_bcd_time(self) -> Self {
        u16::from(u8::from_bcd((self >> 8) as u8) * 60) +
        u16::from(u8::from_bcd(self as u8))
    }

    #[inline]
    fn to_bcd_time(self) -> Self {
        (u16::from(u8::to_bcd((self / 60 % 24) as u8)) << 8) +
        u16::from(u8::to_bcd((self % 60) as u8))
    }
}


/// Converts u32 bcd to seconds
impl BCDTime for u32 {
    #[inline]
    fn from_bcd_time(self) -> Self {
        (u32::from(u8::from_bcd((self >> 16) as u8)) * 3600) +
        (u32::from(u8::from_bcd((self >> 8) as u8)) * 60) +
        u32::from(u8::from_bcd(self as u8))
    }

    #[inline]
    fn to_bcd_time(self) -> Self {
        (u32::from(u8::to_bcd((self / 3600 % 24) as u8)) << 16) +
        (u32::from(u8::to_bcd((self / 60 % 60) as u8)) << 8) +
        u32::from(u8::to_bcd((self % 60) as u8))
    }
}
