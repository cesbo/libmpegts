/// Converts between Unix Timestamp and Modified Julian Date
pub trait MJD {
    fn from_mjd(value: u64) -> u16;
    fn to_mjd(self) -> u64;
}

impl MJD for u16 {
    #[inline]
    fn from_mjd(value: u64) -> u16 {
        (value / 86400 + 40587) as u16
    }

    #[inline]
    fn to_mjd(self) -> u64 {
        debug_assert!(self >= 40587);
        (u64::from(self) - 40587) * 86400
    }
}
