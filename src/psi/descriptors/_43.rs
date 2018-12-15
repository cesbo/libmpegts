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
    pub s2: bool,
    pub modulation: u8,
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
        let s2 = ((slice[8] & 0b0000_0100) >> 2) == 1;

        Self {
            frequency: u32::from_bcd(base::get_u32(&slice[2 ..])) * 10,
            orbital_position: u16::from_bcd(base::get_u16(&slice[6 ..])) * 6,
            west_east_flag: (slice[8] & 0b1000_0000) >> 7,
            polarization: (slice[8] & 0b0110_0000) >> 5,
            rof: if s2 { (slice[8] & 0b0001_1000) >> 3 } else { 0 },
            s2,
            modulation: slice[8] & 0b0000_0011,
            symbol_rate: u32::from_bcd(base::get_u32(&slice[9 ..]) >> 4),
            fec: slice[12] & 0x0F
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {

    }
}
