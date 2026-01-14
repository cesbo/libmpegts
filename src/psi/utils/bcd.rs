/// BCD (Binary-Coded Decimal) is a class of binary encodings of
/// decimal numbers where each decimal digit is represented
/// by a fixed number of bits.
pub trait Bcd<T>: Sized {
    fn from_bcd(value: T) -> Self;
    fn into_bcd(self) -> Self;
}

impl Bcd<u8> for u8 {
    #[inline]
    fn from_bcd(value: u8) -> Self {
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

impl Bcd<[u8; 2]> for u16 {
    #[inline]
    fn from_bcd(value: [u8; 2]) -> Self {
        u16::from(u8::from_bcd(value[0])) * 100 + u16::from(u8::from_bcd(value[1]))
    }

    #[inline]
    fn into_bcd(self) -> Self {
        (u16::from(((self / 100) as u8).into_bcd()) << 8)
            + u16::from(((self % 100) as u8).into_bcd())
    }
}

impl Bcd<[u8; 4]> for u32 {
    #[inline]
    fn from_bcd(value: [u8; 4]) -> Self {
        (u32::from(u16::from_bcd([value[0], value[1]])) * 10000)
            + u32::from(u16::from_bcd([value[2], value[3]]))
    }

    #[inline]
    fn into_bcd(self) -> Self {
        (u32::from(((self / 10000) as u16).into_bcd()) << 16)
            + u32::from(((self % 10000) as u16).into_bcd())
    }
}

/// Converts between Unix Timestamp and Binary Coded Decimal Time
pub trait BcdTime<T>: Sized {
    fn from_bcd_time(value: T) -> Self;
    fn into_bcd_time(self) -> Self;
}

/// Converts u16 bcd to minutes
impl BcdTime<[u8; 2]> for u16 {
    #[inline]
    fn from_bcd_time(value: [u8; 2]) -> Self {
        u16::from(u8::from_bcd(value[0]) * 60) + u16::from(u8::from_bcd(value[1]))
    }

    #[inline]
    fn into_bcd_time(self) -> Self {
        (u16::from(((self / 60 % 24) as u8).into_bcd()) << 8)
            + u16::from(((self % 60) as u8).into_bcd())
    }
}

/// Converts u32 bcd to seconds
impl BcdTime<[u8; 3]> for u32 {
    #[inline]
    fn from_bcd_time(value: [u8; 3]) -> Self {
        u32::from(u8::from_bcd(value[0])) * 3600
            + u32::from(u8::from_bcd(value[1])) * 60
            + u32::from(u8::from_bcd(value[2]))
    }

    #[inline]
    fn into_bcd_time(self) -> Self {
        (u32::from(((self / 3600 % 24) as u8).into_bcd()) << 16)
            + (u32::from(((self / 60 % 60) as u8).into_bcd()) << 8)
            + u32::from(((self % 60) as u8).into_bcd())
    }
}
