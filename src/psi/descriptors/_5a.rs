use crate::base;

/// Terrestrial delivery system descriptor.
///
/// EN 300 468 - 6.2.13.4
#[derive(Debug, Default)]
pub struct Desc5A {
    /// Frequency in Hz.
    pub frequency: u32,
    /// Used bandwidth.
    pub bandwidth: u8,
    /// Stream's hierarchical priority.
    /// * `true`  - associated TS is a HP (high priority) stream
    /// * `false` - associated TS is a LP (low priority) stream
    pub priority: bool,
    /// Usage of time slicing.
    /// * `true`  - Time Slicing is not used.
    /// * `false` - at least one elementary stream uses Time Slicing
    pub time_slicing: bool,
    /// Usage of the MPE-FEC.
    /// * `true`  - MPE-FEC is not used
    /// * `false` - at least one elementary stream uses MPE-FEC
    pub mpe_fec: bool,
    /// Modulation scheme used on a terrestrial delivery system.
    pub modulation: u8,
    /// Specifies whether the transmission is hierarchical and,
    /// if so, what the α value is.
    pub hierarchy: u8,
    /// HP stream inner FEC scheme.
    pub code_rate_hp: u8,
    /// LP stream inner FEC scheme.
    pub code_rate_lp: u8,
    /// Guard interval value.
    pub guard_interval: u8,
    /// Number of carriers in an OFDM frame.
    pub transmission: u8,
    /// Indicates whether other frequencies are in use.
    /// * `true`  - one or more other frequencies are in use
    /// * `false` - no other frequency is in use
    pub other_frequency_flag: bool
}

impl Desc5A {
    #[inline]
    pub fn min_size() -> usize {
        13
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() == Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: base::get_u32(&slice[2 ..]) * 10,
            bandwidth: (slice[6] & 0b1110_0000) >> 5,
            priority: ((slice[6] & 0b0001_0000) >> 4) == 1,
            time_slicing: ((slice[6] & 0b0000_1000) >> 3) == 1,
            mpe_fec: ((slice[6] & 0b0000_0100) >> 2) == 1,
            modulation: (slice[7] & 0b1100_0000) >> 6,
            hierarchy: (slice[7] & 0b0011_1000) >> 3,
            code_rate_hp: slice[7] & 0b0000_0111,
            code_rate_lp: (slice[8] & 0b1110_0000) >> 5,
            guard_interval: (slice[8] & 0b0001_1000) >> 3,
            transmission: (slice[8] & 0b0000_0110) >> 1,
            other_frequency_flag: (slice[8] & 0b0000_0001) == 1
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x5a);
        buffer.push((Self::min_size() - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u32(&mut buffer[skip ..], self.frequency / 10);
        buffer.push(
            (self.bandwidth << 5) |
            ((self.priority as u8) << 4) |
            ((self.time_slicing as u8) << 3) |
            ((self.mpe_fec as u8) << 2) |
            0b0000_0011  // reserved
        );
        buffer.push(
            (self.modulation << 6) |
            (self.hierarchy << 3) |
            self.code_rate_hp
        );
        buffer.push(
            (self.code_rate_lp << 5) |
            (self.guard_interval << 3) |
            (self.transmission << 1) |
            (self.other_frequency_flag as u8)
        );

        let skip = buffer.len();
        buffer.resize(skip + 4, 0xff);  // reserved
    }
}
