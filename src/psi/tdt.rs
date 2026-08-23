use crate::{
    pack_bits,
    psi::{
        PsiSectionError,
        Sections,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
        MjdTo,
    },
};

/// TS Packet Identifier for TDT
pub const TDT_PID: u16 = 0x0014;

const TDT_TABLE_ID: u8 = 0x70;
const TDT_SECTION_SIZE: usize = 8;

pub struct TdtSectionRef<'a>(&'a [u8]);

impl<'a> TdtSectionRef<'a> {
    /// Current time and date in UTC
    pub fn time(&self) -> u64 {
        u64::from_mjd([self.0[3], self.0[4]])
            + u32::from_bcd_time([self.0[5], self.0[6], self.0[7]]) as u64
    }
}

impl<'a> TryFrom<&'a [u8]> for TdtSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < TDT_SECTION_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != TDT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        Ok(TdtSectionRef(&value[.. section_length]))
    }
}

/// TDT section generation config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TdtConfig {
    /// Current time and date in UTC as a Unix timestamp
    pub time: u64,
}

/// One-shot TDT (Time and Date Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{TdtBuilder, TdtConfig, TdtSectionRef};
///
/// let sections = TdtBuilder::build(TdtConfig {
///     time: 1_800_000_000,
/// });
/// assert_eq!(sections.len(), 1);
/// let tdt = TdtSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(tdt.time(), 1_800_000_000);
/// ```
pub struct TdtBuilder;

impl TdtBuilder {
    /// Converts a TDT config into one finalized PSI section. TDT is a
    /// short-form section and carries no CRC32.
    pub fn build(config: TdtConfig) -> Sections {
        let mut buffer = Vec::with_capacity(TDT_SECTION_SIZE);
        buffer.push(TDT_TABLE_ID);
        buffer.extend_from_slice(&pack_bits!(u16,
            section_syntax_indicator: 1 => 0,
            reserved_future_use: 1 => 1,
            reserved: 2 => 0b11,
            section_length: 12 => (TDT_SECTION_SIZE - 3) as u16,
        ));
        buffer.extend_from_slice(&config.time.into_mjd());
        buffer.extend_from_slice(&config.time.into_bcd_time());

        Sections::new(buffer, vec![0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tdt() {
        // 2027-01-15 08:00:00 UTC, MJD 61420
        let sections = TdtBuilder::build(TdtConfig {
            time: 1_800_000_000,
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            section,
            [0x70, 0x70, 0x05, 0xef, 0xec, 0x08, 0x00, 0x00]
        );

        let tdt = TdtSectionRef::try_from(section).unwrap();
        assert_eq!(tdt.time(), 1_800_000_000);
    }
}
