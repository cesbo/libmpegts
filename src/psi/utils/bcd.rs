/// BCD (Binary-Coded Decimal) is a class of binary encodings of
/// decimal numbers where each decimal digit is represented
/// by a fixed number of bits.
pub trait BCD: Sized {
    fn from_bcd(value: Self) -> Self;
    fn into_bcd(self) -> Self;
}

impl BCD for u8 {
    #[inline]
    fn from_bcd(value: Self) -> Self {
        debug_assert!((value & 0xF0) < 0xA0);
        debug_assert!((value & 0x0F) < 0x0A);
        value - (value >> 4) * 6
    }

    #[inline]
    fn into_bcd(self) -> Self {
        debug_assert!(self < 100);
        self + (self / 10) * 6
    }
}

impl BCD for u16 {
    #[inline]
    fn from_bcd(value: Self) -> Self {
        (u16::from(u8::from_bcd((value >> 8) as u8)) * 100) + u16::from(u8::from_bcd(value as u8))
    }

    #[inline]
    fn into_bcd(self) -> Self {
        (u16::from(((self / 100) as u8).into_bcd()) << 8) + u16::from(((self % 100) as u8).into_bcd())
    }
}

impl BCD for u32 {
    #[inline]
    fn from_bcd(value: Self) -> Self {
        (u32::from(u16::from_bcd((value >> 16) as u16)) * 10000)
            + u32::from(u16::from_bcd(value as u16))
    }

    #[inline]
    fn into_bcd(self) -> Self {
        (u32::from(((self / 10000) as u16).into_bcd()) << 16)
            + u32::from(((self % 10000) as u16).into_bcd())
    }
}

/// Converts between Unix Timestamp and Binary Coded Decimal Time
pub trait BCDTime: Sized {
    fn from_bcd_time(value: Self) -> Self;
    fn into_bcd_time(self) -> Self;
}

/// Converts u16 bcd to minutes
impl BCDTime for u16 {
    #[inline]
    fn from_bcd_time(value: Self) -> Self {
        u16::from(u8::from_bcd((value >> 8) as u8) * 60) + u16::from(u8::from_bcd(value as u8))
    }

    #[inline]
    fn into_bcd_time(self) -> Self {
        (u16::from(((self / 60 % 24) as u8).into_bcd()) << 8)
            + u16::from(((self % 60) as u8).into_bcd())
    }
}

/// Converts u32 bcd to seconds
impl BCDTime for u32 {
    #[inline]
    fn from_bcd_time(value: Self) -> Self {
        (u32::from(u8::from_bcd((value >> 16) as u8)) * 3600)
            + (u32::from(u8::from_bcd((value >> 8) as u8)) * 60)
            + u32::from(u8::from_bcd(value as u8))
    }

    #[inline]
    fn into_bcd_time(self) -> Self {
        (u32::from(((self / 3600 % 24) as u8).into_bcd()) << 16)
            + (u32::from(((self / 60 % 60) as u8).into_bcd()) << 8)
            + u32::from(((self % 60) as u8).into_bcd())
    }
}
