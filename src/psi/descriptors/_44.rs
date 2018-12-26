use base;
use bcd::BCD;


/// Cable delivery system descriptor.
///
/// EN 300 468 - 6.2.13.1
#[derive(Debug, Default)]
pub struct Desc44 {
    /// Frequency in Hz.
    pub frequency: u32,
    /// Outer FEC scheme.
    pub fec_outer: u8,
    /// Modulation scheme used on a cable delivery system.
    pub modulation: u8,
    /// Symbol rate in Ksymbol/s, used on a satellite delivery system.
    pub symbol_rate: u32,
    /// Inner FEC scheme.
    pub fec: u8
}

impl Desc44 {
    #[inline]
    pub fn min_size() -> usize {
        13
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() == Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: u32::from_bcd(base::get_u32(&slice[2 ..])) * 100,
            fec_outer: slice[7] & 0x0F,
            modulation: slice[8],
            symbol_rate: u32::from_bcd(base::get_u32(&slice[9 ..]) >> 8),
            fec: slice[12] & 0x0F
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x44);
        buffer.push((Self::min_size() - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u32(&mut buffer[skip ..], (self.frequency / 100).to_bcd());
        buffer.push(0xFF);  // reserved
        buffer.push(0xF0 | self.fec_outer);  // reserved + fec outer
        buffer.push(self.modulation);

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u32(&mut buffer[skip ..], self.symbol_rate.to_bcd() << 8);
        buffer[skip + 3] |= self.fec;
    }
}
