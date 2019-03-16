use crate::bytes::*;
use crate::bcd::*;


const MIN_SIZE: usize = 13;


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
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: slice[2 ..].get_u32().from_bcd() * 100,
            fec_outer: slice[7] & 0x0F,
            modulation: slice[8],
            symbol_rate: slice[9 ..].get_u24().from_bcd(),
            fec: slice[12] & 0x0F
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        MIN_SIZE
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let size = self.size();
        let skip = buffer.len();
        buffer.resize(skip + size, 0x00);

        buffer[skip] = 0x44;
        buffer[skip + 1] = (size - 2) as u8;
        buffer[skip + 2 ..].set_u32((self.frequency / 100).to_bcd());
        buffer[skip + 6] = 0xFF;  // reserved
        buffer[skip + 7] = 0xF0 | self.fec_outer;  // reserved + fec outer
        buffer[skip + 8] = self.modulation;
        buffer[skip + 9 ..].set_u24(self.symbol_rate.to_bcd());
        buffer[skip + 12] = self.fec;
    }
}
