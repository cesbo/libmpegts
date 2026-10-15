/// Converts between Unix Timestamp and Modified Julian Date
pub trait MjdFrom<T>: Sized {
    fn from_mjd(value: T) -> Self;
}

pub trait MjdTo<T> {
    fn into_mjd(self) -> T;
}

impl MjdFrom<[u8; 2]> for u64 {
    /// A date before 1970-01-01 (MJD 40587) reads as 0: a section without a
    /// CRC (TDT) or a corrupt field can carry any value.
    fn from_mjd(value: [u8; 2]) -> u64 {
        let value = u16::from_be_bytes(value) as u64;
        value.saturating_sub(40587) * 86400
    }
}

impl MjdTo<[u8; 2]> for u64 {
    fn into_mjd(self) -> [u8; 2] {
        ((self / 86400 + 40587) as u16).to_be_bytes()
    }
}
