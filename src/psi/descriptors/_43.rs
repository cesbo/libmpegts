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
    /// Satellite position in the western or eastern part of the orbit.
    pub west_east_flag: u8,
    /// Polarization of the transmitted signal.
    pub polarization: u8,
    /// Roll-off factor used in DVB-S2.
    pub rof: u8,
    /// Broadcast scheme used on a satellite delivery system.
    /// DVB-S2 or not.
    pub s2: bool,
    /// Modulation scheme used on a satellite delivery system.
    pub modulation: u8,
    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    pub symbol_rate: u32,
    /// Inner FEC scheme.
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
        Self {
            frequency: u32::from_bcd(base::get_u32(&slice[2 ..])) * 10,
            orbital_position: u16::from_bcd(base::get_u16(&slice[6 ..])) * 6,
            west_east_flag: (slice[8] & 0b1000_0000) >> 7,
            polarization: (slice[8] & 0b0110_0000) >> 5,
            rof: (slice[8] & 0b0001_1000) >> 3,
            s2: ((slice[8] & 0b0000_0100) >> 2) == 1,
            modulation: slice[8] & 0b0000_0011,
            symbol_rate: u32::from_bcd(base::get_u32(&slice[9 ..]) >> 8),
            fec: slice[12] & 0x0F
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x43);
        buffer.push((Self::min_size() - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 6, 0x00);
        base::set_u32(&mut buffer[skip ..], (self.frequency / 10).to_bcd());
        base::set_u16(&mut buffer[skip + 4 ..], (self.orbital_position / 6).to_bcd());
        buffer.push(
            (self.west_east_flag << 7) |
            (self.polarization << 5) |
            (self.rof << 3) |
            ((self.s2 as u8) << 2) |
            self.modulation
        );

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u32(&mut buffer[skip ..], self.symbol_rate.to_bcd() << 8);
        buffer[skip + 3] |= self.fec;
    }
}
