use core::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Maximum valid value (33 bits)
    pub const MAX: u64 = (1 << 33) - 1;

    /// 90 kHz clock per millisecond
    pub const CLOCK_MS: u64 = 90;

    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value & Self::MAX)
    }

    #[inline]
    pub const fn value(&self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn wrapping_add(self, v: Self) -> Self {
        Self(self.0.wrapping_add(v.0) & Self::MAX)
    }

    #[inline]
    pub const fn wrapping_sub(self, v: Self) -> Self {
        Self(self.0.wrapping_sub(v.0) & Self::MAX)
    }

    /// Circular comparison for 33-bit wrapping timestamps.
    pub fn wrapping_cmp(self, other: Self) -> Ordering {
        let delta = other.wrapping_sub(self).value();
        if delta == 0 {
            Ordering::Equal
        } else if delta < (1 << 32) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    /// Writes the timestamp into a 5-byte buffer with the given marker (PTS or DTS)
    /// Returns the number of bytes written (always 5)
    ///
    /// Markers:
    /// - 0b0011 for PTS with following DTS
    /// - 0b0010 for PTS only
    /// - 0b0001 for DTS only
    pub fn write(&self, buf: &mut [u8], marker: u8) -> usize {
        buf[0] = (marker << 4) | ((self.0 >> 29) & 0x0E) as u8 | 0x01;
        buf[1] = (self.0 >> 22) as u8;
        buf[2] = ((self.0 >> 14) & 0xFE) as u8 | 0x01;
        buf[3] = (self.0 >> 7) as u8;
        buf[4] = ((self.0 << 1) & 0xFE) as u8 | 0x01;
        5
    }
}

impl From<u64> for Timestamp {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Timestamp> for u64 {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtsDts {
    pub pts: Timestamp,
    pub dts: Option<Timestamp>,
}

impl PtsDts {
    pub fn new(pts: impl Into<Timestamp>) -> Self {
        Self {
            pts: pts.into(),
            dts: None,
        }
    }

    pub fn with_dts(mut self, dts: impl Into<Timestamp>) -> Self {
        self.dts = Some(dts.into());
        self
    }

    /// Returns DTS if present, otherwise PTS.
    /// Used for scheduling (decode order).
    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.dts.unwrap_or(self.pts)
    }
}

impl From<(u64, Option<u64>)> for PtsDts {
    fn from((pts, dts): (u64, Option<u64>)) -> Self {
        Self {
            pts: pts.into(),
            dts: dts.map(Timestamp::from),
        }
    }
}
