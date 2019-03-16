use crate::bytes::*;


const MIN_SIZE: usize = 13;


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
    /// * `1`  - associated TS is a HP (high priority) stream
    /// * `0` - associated TS is a LP (low priority) stream
    pub priority: u8,
    /// Usage of time slicing.
    /// * `1`  - Time Slicing is not used.
    /// * `0` - at least one elementary stream uses Time Slicing
    pub time_slicing: u8,
    /// Usage of the MPE-FEC.
    /// * `1`  - MPE-FEC is not used
    /// * `0` - at least one elementary stream uses MPE-FEC
    pub mpe_fec: u8,
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
    /// * `1`  - one or more other frequencies are in use
    /// * `0` - no other frequency is in use
    pub other_frequency_flag: u8
}


impl Desc5A {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: slice[2 ..].get_u32() * 10,
            bandwidth: (slice[6] & 0b1110_0000) >> 5,
            priority: (slice[6] & 0b0001_0000) >> 4,
            time_slicing: (slice[6] & 0b0000_1000) >> 3,
            mpe_fec: (slice[6] & 0b0000_0100) >> 2,
            modulation: (slice[7] & 0b1100_0000) >> 6,
            hierarchy: (slice[7] & 0b0011_1000) >> 3,
            code_rate_hp: slice[7] & 0b0000_0111,
            code_rate_lp: (slice[8] & 0b1110_0000) >> 5,
            guard_interval: (slice[8] & 0b0001_1000) >> 3,
            transmission: (slice[8] & 0b0000_0110) >> 1,
            other_frequency_flag: slice[8] & 0b0000_0001
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        MIN_SIZE
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x5a);
        buffer.push((MIN_SIZE - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 11, 0x00);
        buffer[skip ..].set_u32(self.frequency / 10);
        buffer[skip + 4] = set_bits!(8,
            self.bandwidth, 3,
            self.priority, 1,
            self.time_slicing, 1,
            self.mpe_fec, 1,
            0b0000_0011, 2);

        buffer[skip + 5] = set_bits!(8,
            self.modulation, 2,
            self.hierarchy, 3,
            self.code_rate_hp, 3);

        buffer[skip + 6] = set_bits!(8,
            self.code_rate_lp, 3,
            self.guard_interval, 2,
            self.transmission, 2,
            self.other_frequency_flag, 1);

        buffer[skip + 7 ..].set_u32(0xFFFF_FFFF);
    }
}
