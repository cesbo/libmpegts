/// Converts between Unix Timestamp and Modified Julian Date
pub trait MJDFrom {
    fn from_mjd(self) -> u64;
}

pub trait MJDTo {
    fn to_mjd(self) -> u16;
}

impl MJDFrom for u16 {
    #[inline]
    fn from_mjd(self) -> u64 {
        debug_assert!(self >= 40587);
        (u64::from(self) - 40587) * 86400
    }
}

impl MJDTo for u64 {
    #[inline]
    fn to_mjd(self) -> u16 {
        (self / 86400 + 40587) as u16
    }
}
