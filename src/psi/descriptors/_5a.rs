use base;


/// Terrestrial delivery system descriptor.
///
/// EN 300 468 - 6.2.13.4
#[derive(Debug, Default)]
pub struct Desc5A {
    pub frequency: u32,
    pub bandwidth: u8,
    pub priority: bool,
    pub time_slicing: bool,
    pub mpe_fec: bool,
    pub modulation: u8,
    pub hierarchy: u8,
    pub code_rate_hp: u8,
    pub code_rate_lp: u8,
    pub guard_interval: u8,
    pub transmission: u8,
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

    pub fn assemble(&self, buffer: &mut Vec<u8>) {

    }
}
