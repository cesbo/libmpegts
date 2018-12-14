use base;
use bcd::BCD;


/// Satellite delivery system descriptor.
///
/// EN 300 468 - 6.2.13.2
#[derive(Debug, Default)]
pub struct Desc43 {
    /// Frequency in KHz.
    pub frequency: u32,
    /// Position in minutes of angle.
    pub orbital_position: u16,
    pub west_east_flag: u8,
    pub polarization: u8,
    pub rof: u8,
    pub modulation_system: u8,
    pub modulation_type: u8,
    pub symbol_rate: u32,
    pub fec: u8
}

impl Desc43 {
    #[inline]
    pub fn min_size() -> usize {
        13
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() == Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        let modulation_system = (slice[8] & 0b_0000_0100) >> 2;
        let mut rof: u8 = 0;
        if modulation_system == 1 {
            rof = (slice[8] & 0b_0001_1000) >> 3;
        }

        Self {
            frequency: u32::from_bcd(base::get_u32(&slice[2 ..])) * 10,
            orbital_position: u16::from_bcd(base::get_u16(&slice[6 ..])) * 6,
            west_east_flag: (slice[8] & 0b_1000_0000) >> 7,
            polarization: (slice[8] & 0b_0110_0000) >> 5,
            rof,
            modulation_system,
            modulation_type: slice[8] & 0b_0000_0011,
            symbol_rate: u32::from_bcd(base::get_u32(&slice[9 ..]) >> 4),
            fec: slice[12] & 0x0F
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {

    }
}
