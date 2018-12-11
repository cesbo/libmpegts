use base;


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
    pub modulation_system: u8,
    pub symbol_rate: u16,
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
        let mut frequency: i32 = 0;
        for i in 2 .. 6 {
            frequency += base::bcd_to_u32(slice[i])
        }
        frequency *= 10;

        let mut orbital_position: i32 = 0;
        for i in 6 .. 8 {
            orbital_position += base::bcd_to_u32(slice[i])
        }
        orbital_position *= 6;

        let modulation_system = (slice[8] & 0b_0000_0100) >> 2;
        if modulation_system == 1 {
            let rof = (slice[8] & 0b_0001_1000) >> 3;
        } else {
            let rof = 0;
        }

        Self::default()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {

    }
}
