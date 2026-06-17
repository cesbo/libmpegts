/// BCD (Binary-Coded Decimal) is a class of binary encodings of
/// decimal numbers where each decimal digit is represented
/// by a fixed number of bits.
pub trait Bcd<T>: Sized {
    fn from_bcd(value: T) -> Self;
    fn into_bcd(self) -> T;
}

impl Bcd<u8> for u8 {
    fn from_bcd(value: u8) -> Self {
        value - (value >> 4) * 6
    }

    fn into_bcd(self) -> u8 {
        self + (self / 10) * 6
    }
}

impl Bcd<[u8; 2]> for u16 {
    fn from_bcd(value: [u8; 2]) -> Self {
        u16::from(u8::from_bcd(value[0])) * 100 + u16::from(u8::from_bcd(value[1]))
    }

    fn into_bcd(self) -> [u8; 2] {
        [
            ((self / 100) as u8).into_bcd(),
            ((self % 100) as u8).into_bcd(),
        ]
    }
}

impl Bcd<[u8; 4]> for u32 {
    fn from_bcd(value: [u8; 4]) -> Self {
        (u32::from(u16::from_bcd([value[0], value[1]])) * 10000)
            + u32::from(u16::from_bcd([value[2], value[3]]))
    }

    fn into_bcd(self) -> [u8; 4] {
        let h = ((self / 10000) as u16).into_bcd();
        let l = ((self % 10000) as u16).into_bcd();
        [h[0], h[1], l[0], l[1]]
    }
}

/// Converts between Unix Timestamp and Binary Coded Decimal Time
pub trait BcdTime<T>: Sized {
    fn from_bcd_time(value: T) -> Self;
    fn into_bcd_time(self) -> T;
}

impl BcdTime<[u8; 2]> for u16 {
    /// Converts BCD time (HH:MM) to minutes
    fn from_bcd_time(value: [u8; 2]) -> Self {
        u16::from(u8::from_bcd(value[0]) * 60) + u16::from(u8::from_bcd(value[1]))
    }

    /// Converts minutes to BCD time
    /// Value should be less than 1440 (24 hours * 60 minutes)
    fn into_bcd_time(self) -> [u8; 2] {
        [
            ((self / 60 % 24) as u8).into_bcd(),
            ((self % 60) as u8).into_bcd(),
        ]
    }
}

impl BcdTime<[u8; 3]> for u32 {
    /// Converts u32 BCD time (HH:MM:SS) to seconds
    fn from_bcd_time(value: [u8; 3]) -> Self {
        u32::from(u8::from_bcd(value[0])) * 3600
            + u32::from(u8::from_bcd(value[1])) * 60
            + u32::from(u8::from_bcd(value[2]))
    }

    /// Converts seconds to BCD time
    fn into_bcd_time(self) -> [u8; 3] {
        let hm = ((self / 60 % 1440) as u16).into_bcd_time();
        let s = ((self % 60) as u8).into_bcd();
        [hm[0], hm[1], s]
    }
}

impl BcdTime<[u8; 3]> for u64 {
    /// Converts u64 BCD time (HH:MM:SS) to seconds
    fn from_bcd_time(value: [u8; 3]) -> Self {
        u64::from(u8::from_bcd(value[0])) * 3600
            + u64::from(u8::from_bcd(value[1])) * 60
            + u64::from(u8::from_bcd(value[2]))
    }

    /// Converts seconds to BCD time
    fn into_bcd_time(self) -> [u8; 3] {
        let hm = ((self / 60 % 1440) as u16).into_bcd_time();
        let s = ((self % 60) as u8).into_bcd();
        [hm[0], hm[1], s]
    }
}
