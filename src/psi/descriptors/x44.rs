use super::Desc;
use crate::utils::Bcd;

const MIN_SIZE: usize = 13;

/// Cable delivery system descriptor.
///
/// EN 300 468 - 6.2.13.1
#[derive(Debug, Default, Clone)]
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
    pub fec: u8,
}

impl Desc44 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            frequency: u32::from_bcd([slice[2], slice[3], slice[4], slice[5]]) * 100,
            fec_outer: slice[7] & 0x0F,
            modulation: slice[8],
            symbol_rate: u32::from_bcd([0, slice[9], slice[10], slice[11]]),
            fec: slice[12] & 0x0F,
        }
    }
}

impl Desc for Desc44 {
    #[inline]
    fn tag(&self) -> u8 {
        0x44
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x44);
        buffer.push((self.size() - 2) as u8);
        buffer.extend_from_slice(&(self.frequency / 100).into_bcd());
        buffer.push(0xFF); // reserved
        buffer.push(0xF0 | self.fec_outer); // reserved + fec outer
        buffer.push(self.modulation);
        buffer.extend_from_slice(&self.symbol_rate.into_bcd()[1 ..]);
        buffer.push(self.fec);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants,
        psi::{
            Desc44,
            Descriptors,
        },
    };

    static DATA_44: &[u8] = &[
        0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0, 0x05, 0x00, 0x68, 0x75, 0x00,
    ];

    #[test]
    fn test_44_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_44);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc44>();
        assert_eq!(desc.frequency, 346000000);
        assert_eq!(desc.fec_outer, constants::FEC_OUTER_NOT_DEFINED);
        assert_eq!(desc.modulation, constants::MODULATION_DVB_C_256_QAM);
        assert_eq!(desc.symbol_rate, 6875);
        assert_eq!(desc.fec, constants::FEC_NOT_DEFINED);
    }

    #[test]
    fn test_44_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc44 {
            frequency: 346000000,
            fec_outer: constants::FEC_OUTER_NOT_DEFINED,
            modulation: constants::MODULATION_DVB_C_256_QAM,
            symbol_rate: 6875,
            fec: constants::FEC_NOT_DEFINED,
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_44);
    }
}
