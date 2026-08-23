use crate::{
    pack_bits,
    psi::{
        DescriptorsRef,
        Psi,
        PsiSectionError,
        Sections,
        check_crc32,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
        MjdTo,
        crc32b,
    },
};

/// TS Packet Identifier for TOT
pub const TOT_PID: u16 = 0x0014;

const TOT_TABLE_ID: u8 = 0x73;
const TOT_HEADER_SIZE: usize = 10;
const TOT_CRC_SIZE: usize = 4;
const TOT_SECTION_SIZE: usize = 1024;

/// Time Offset Table carries the UTC-time and date information and local time offset
pub struct TotSectionRef<'a>(&'a [u8]);

impl<'a> TotSectionRef<'a> {
    /// Table ID
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// Current time and date in UTC
    pub fn time(&self) -> u64 {
        u64::from_mjd([self.0[3], self.0[4]])
            + u32::from_bcd_time([self.0[5], self.0[6], self.0[7]]) as u64
    }

    fn descriptors_length(&self) -> usize {
        (u16::from_be_bytes([self.0[8], self.0[9]]) & 0x0fff) as usize
    }

    /// List of descriptors.
    pub fn descriptors(&self) -> Option<DescriptorsRef<'_>> {
        let descriptors_len = self.descriptors_length();
        (descriptors_len > 0)
            .then(|| self.0[TOT_HEADER_SIZE .. TOT_HEADER_SIZE + descriptors_len].into())
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - TOT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for TotSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < TOT_HEADER_SIZE + TOT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != TOT_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(TotSectionRef(&value[.. section_length]))
    }
}

impl<'a> TryFrom<&'a Psi> for TotSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(psi: &'a Psi) -> Result<Self, Self::Error> {
        match psi.payload() {
            Some(payload) => TotSectionRef::try_from(payload),
            None => Err(PsiSectionError::InvalidSectionLength),
        }
    }
}

/// TOT section generation config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotConfig {
    /// Current time and date in UTC as a Unix timestamp
    pub time: u64,
    /// Raw descriptor bytes for the section body
    pub descriptors: Vec<u8>,
}

/// One-shot TOT (Time Offset Table) section generator.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{Desc58, Desc58Item, Descriptor, TotBuilder, TotConfig, TotSectionRef};
///
/// let mut descriptors = Vec::new();
/// Desc58 {
///     items: &[Desc58Item {
///         country_code: *b"BUL",
///         country_region_id: 0,
///         local_time_offset_polarity: false,
///         local_time_offset: 120,
///         time_of_change: 1_800_000_000,
///         next_time_offset: 180,
///     }],
/// }
/// .encode(&mut descriptors)
/// .unwrap();
///
/// let sections = TotBuilder::build(TotConfig {
///     time: 1_800_000_000,
///     descriptors,
/// });
/// assert_eq!(sections.len(), 1);
/// let tot = TotSectionRef::try_from(&sections[0][..]).unwrap();
/// assert_eq!(tot.time(), 1_800_000_000);
/// ```
pub struct TotBuilder;

impl TotBuilder {
    /// Converts a TOT config into one finalized PSI section. TOT is a
    /// short-form section but carries a CRC32.
    pub fn build(config: TotConfig) -> Sections {
        let descriptors_length = config.descriptors.len();
        debug_assert!(TOT_HEADER_SIZE + descriptors_length + TOT_CRC_SIZE <= TOT_SECTION_SIZE);

        let mut buffer = Vec::with_capacity(TOT_HEADER_SIZE + descriptors_length + TOT_CRC_SIZE);
        buffer.push(TOT_TABLE_ID);
        buffer.extend_from_slice(&pack_bits!(u16,
            section_syntax_indicator: 1 => 0,
            reserved_future_use: 1 => 1,
            reserved: 2 => 0b11,
            section_length: 12 => (TOT_HEADER_SIZE - 3 + descriptors_length + TOT_CRC_SIZE) as u16,
        ));
        buffer.extend_from_slice(&config.time.into_mjd());
        buffer.extend_from_slice(&config.time.into_bcd_time());
        buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 4 => 0b1111,
            descriptors_loop_length: 12 => descriptors_length as u16,
        ));
        buffer.extend_from_slice(&config.descriptors);

        let crc = crc32b(&buffer);
        buffer.extend_from_slice(&crc.to_be_bytes());

        Sections::new(buffer, vec![0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::{
        Desc58,
        Desc58Item,
        Desc58Ref,
        Descriptor,
    };

    #[test]
    fn builds_empty_tot() {
        // 2027-01-15 08:00:00 UTC, MJD 61420
        let sections = TotBuilder::build(TotConfig {
            time: 1_800_000_000,
            descriptors: Vec::new(),
        });

        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(section.len(), TOT_HEADER_SIZE + TOT_CRC_SIZE);
        assert_eq!(
            &section[.. TOT_HEADER_SIZE],
            [0x73, 0x70, 0x0b, 0xef, 0xec, 0x08, 0x00, 0x00, 0xf0, 0x00]
        );

        let tot = TotSectionRef::try_from(section).unwrap();
        assert_eq!(tot.table_id(), 0x73);
        assert_eq!(tot.time(), 1_800_000_000);
        assert!(tot.descriptors().is_none());
    }

    #[test]
    fn builds_tot_with_local_time_offset() {
        let item = Desc58Item {
            country_code: *b"BUL",
            country_region_id: 0,
            local_time_offset_polarity: false,
            local_time_offset: 120,
            time_of_change: 1_806_800_400,
            next_time_offset: 180,
        };

        let mut descriptors = Vec::new();
        Desc58 { items: &[item] }.encode(&mut descriptors).unwrap();

        let sections = TotBuilder::build(TotConfig {
            time: 1_800_000_000,
            descriptors,
        });

        let tot = TotSectionRef::try_from(&sections[0][..]).unwrap();
        assert_eq!(tot.time(), 1_800_000_000);

        let desc = tot
            .descriptors()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        let offsets = Desc58Ref::try_from(desc).unwrap();
        assert_eq!(offsets.items().next().unwrap(), item);
    }
}
