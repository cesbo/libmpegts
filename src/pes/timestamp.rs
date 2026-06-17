#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Maximum valid value (33 bits)
    pub const MAX: u64 = (1 << 33) - 1;

    /// 90 kHz clock per millisecond
    pub const CLOCK_MS: u64 = 90;

    pub const fn new(value: u64) -> Self {
        Self(value & Self::MAX)
    }

    pub const fn value(&self) -> u64 {
        self.0
    }

    pub const fn wrapping_add(self, v: Self) -> Self {
        Self(self.0.wrapping_add(v.0) & Self::MAX)
    }

    pub const fn wrapping_sub(self, v: Self) -> Self {
        Self(self.0.wrapping_sub(v.0) & Self::MAX)
    }

    /// Circular comparison.
    /// Returns true if self is before other
    pub fn is_before(self, other: Self) -> bool {
        let diff = self.wrapping_sub(other).value();
        diff > (Self::MAX >> 1)
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

    /// Reads a timestamp from a 5-byte PES timestamp field with the expected marker.
    pub fn read(buf: &[u8], marker: u8) -> Option<Self> {
        if buf.len() < 5 {
            return None;
        }

        if (buf[0] >> 4) != marker
            || (buf[0] & 0x01) == 0
            || (buf[2] & 0x01) == 0
            || (buf[4] & 0x01) == 0
        {
            return None;
        }

        let b0 = ((buf[0] & 0x0E) >> 1) as u64;
        let b1 = buf[1] as u64;
        let b2 = ((buf[2] & 0xFE) >> 1) as u64;
        let b3 = buf[3] as u64;
        let b4 = ((buf[4] & 0xFE) >> 1) as u64;

        Some(Self::new(
            (b0 << 30) | (b1 << 22) | (b2 << 15) | (b3 << 7) | b4,
        ))
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
