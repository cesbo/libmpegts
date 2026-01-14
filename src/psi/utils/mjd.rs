/// Converts between Unix Timestamp and Modified Julian Date
pub trait MJDFrom: Sized {
    fn from_mjd(value: Self) -> u64;
}

pub trait MJDTo {
    fn into_mjd(self) -> u16;
}

impl MJDFrom for u16 {
    #[inline]
    fn from_mjd(value: Self) -> u64 {
        debug_assert!(value >= 40587);
        (u64::from(value) - 40587) * 86400
    }
}

impl MJDTo for u64 {
    #[inline]
    fn into_mjd(self) -> u16 {
        (self / 86400 + 40587) as u16
    }
}
