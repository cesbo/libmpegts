use super::{
    PSI_CRC_SIZE,
    psi_section_length,
};

// Long form header, iso13818-1 Table 2-30:
// table_id .. last_section_number
const SECTION_HEADER_SIZE: usize = 8;
// Long form header with no payload plus CRC_32
const SECTION_MIN_SIZE: usize = SECTION_HEADER_SIZE + PSI_CRC_SIZE;
// 3 bytes before section_length plus its limit for private sections
// (e.g. EIT), 2.4.4.10; PAT, PMT and CAT are limited to 3 + 1021
const SECTION_MAX_SIZE: usize = 3 + 4093;
// section_number is u8, so a table has at most 256 sections
const MAX_SECTIONS: usize = u8::MAX as usize + 1;

/// Tracks the sections of one PSI table to detect its changes.
///
/// The table is identified by `table_id` and `table_id_extension` of the
/// first accepted section (iso13818-1 2.4.4.11). For every `section_number`
/// the CRC_32 of the last received section is stored, so a repeated section is
/// reported as [`Change::Unchanged`] and a modified one as [`Change::Updated`].
///
/// ```
/// use libmpegts::psi::{
///     Change,
///     SectionTable,
/// };
///
/// // PAT: transport_stream_id 1, version 0, single section
/// let section = [
///     0x00, 0xb0, 0x0d, 0x00, 0x01, 0xc1, 0x00, 0x00, // header
///     0x00, 0x01, 0xe1, 0x00, // program 1 -> PMT PID 0x100
///     0x12, 0x34, 0x56, 0x78, // CRC_32
/// ];
///
/// let mut table = SectionTable::new();
/// assert_eq!(table.push(&section), Change::Updated);
/// assert_eq!(table.push(&section), Change::Unchanged);
/// ```
pub struct SectionTable {
    table_id: u8,
    table_id_extension: u16,
    version: u8,
    last_section_number: u8,
    crc: [Option<u32>; MAX_SECTIONS],
    is_empty: bool,
}

/// Result of [`SectionTable::push`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Section is not accepted: another table, malformed or not applicable
    /// section. The stored state is not changed
    Ignored,
    /// Section is already stored with the same CRC_32
    Unchanged,
    /// New section or a section with another CRC_32
    Updated,
    /// `version_number` or `last_section_number` changed: all stored sections
    /// are dropped and the table starts over from this section
    Reset,
}

impl SectionTable {
    /// Creates an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one complete section, from `table_id` through CRC_32.
    ///
    /// The first accepted section defines the table. Returns
    /// [`Change::Ignored`] for:
    /// - short form sections (`section_syntax_indicator == 0`)
    /// - invalid `section_length` or a buffer shorter than the section
    /// - `current_next_indicator == 0`
    /// - `section_number` greater than `last_section_number`
    /// - another `table_id` or `table_id_extension`
    ///
    /// CRC_32 is not verified: pass sections already checked with
    /// [`check_crc32`](super::check_crc32) or a `*SectionRef::try_from`.
    pub fn push(&mut self, section: &[u8]) -> Change {
        if section.len() < SECTION_MIN_SIZE {
            return Change::Ignored;
        }

        // iso13818-1 Table 2-30: only the long form (section_syntax_indicator == 1)
        // has table_id_extension, version_number, section numbers and CRC_32
        if section[1] & 0x80 == 0 {
            return Change::Ignored;
        }

        // full section size: 3 header bytes + section_length
        let end = psi_section_length(section);
        if !(SECTION_MIN_SIZE ..= SECTION_MAX_SIZE).contains(&end) || section.len() < end {
            return Change::Ignored;
        }

        // current_next_indicator == 0: not yet applicable
        if section[5] & 0x01 == 0 {
            return Change::Ignored;
        }

        let last_section_number = section[7];
        // iso13818-1 2.4.4.11: last_section_number is the highest section_number
        // of the table, so a greater section_number means a broken section
        // (corrupted data or a faulty multiplexer). Ignore it so that it does not
        // leave a stale crc slot beyond last_section_number.
        let section_number = section[6];
        if section_number > last_section_number {
            return Change::Ignored;
        }

        let table_id = section[0];
        let table_id_extension = u16::from_be_bytes([section[3], section[4]]);
        let version = (section[5] >> 1) & 0x1F;
        let p = &section[end - PSI_CRC_SIZE .. end];
        let crc = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);

        let mut reset = false;

        if !self.is_empty {
            // another table
            if self.table_id != table_id || self.table_id_extension != table_id_extension {
                return Change::Ignored;
            }

            // new version of the table: drop all stored sections and start over
            if self.version != version || self.last_section_number != last_section_number {
                self.clear();
                reset = true;
            }
        }

        if self.is_empty {
            self.table_id = table_id;
            self.table_id_extension = table_id_extension;
            self.version = version;
            self.last_section_number = last_section_number;
            self.is_empty = false;
        }

        // Stores the new crc and returns the previous one
        let previous = self.crc[usize::from(section_number)].replace(crc);

        if reset {
            Change::Reset
        } else if previous == Some(crc) {
            Change::Unchanged
        } else {
            Change::Updated
        }
    }

    /// Drops all stored sections, the next accepted section defines the table
    /// again.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for SectionTable {
    fn default() -> Self {
        Self {
            table_id: 0,
            table_id_extension: 0,
            version: 0,
            last_section_number: 0,
            crc: [None; MAX_SECTIONS],
            is_empty: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a long form section (iso13818-1 Table 2-30).
    /// crc is stored as is, without real CRC_32 calculation.
    fn section(
        table_id: u8,
        table_id_extension: u16,
        version: u8,
        section_number: u8,
        last_section_number: u8,
        crc: u32,
    ) -> Vec<u8> {
        let payload = [0x11, 0x22, 0x33];
        let section_length = 9 + payload.len();

        let mut s = vec![
            table_id,
            0xB0 | ((section_length >> 8) as u8 & 0x0F),
            section_length as u8,
        ];
        s.extend(table_id_extension.to_be_bytes());
        s.push(0xC0 | ((version & 0x1F) << 1) | 0x01);
        s.push(section_number);
        s.push(last_section_number);
        s.extend(payload);
        s.extend(crc.to_be_bytes());
        s
    }

    const EIT: u8 = 0x4E;
    const SERVICE_ID: u16 = 101;

    #[test]
    fn first_section_updated() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
    }

    #[test]
    fn repeated_section_unchanged() {
        let mut table = SectionTable::new();
        let s = section(EIT, SERVICE_ID, 0, 0, 1, 0xA0);
        assert_eq!(table.push(&s), Change::Updated);
        assert_eq!(table.push(&s), Change::Unchanged);
        assert_eq!(table.push(&s), Change::Unchanged);
    }

    #[test]
    fn new_section_number_updated() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 1, 1, 0xA1)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Unchanged
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 1, 1, 0xA1)),
            Change::Unchanged
        );
    }

    #[test]
    fn crc_changed_updated() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xB0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xB0)),
            Change::Unchanged
        );
    }

    #[test]
    fn single_section_table() {
        // last_section_number == 0 is a valid table of one section (e.g. PAT)
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(0x00, 1, 0, 0, 0, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(0x00, 1, 0, 0, 0, 0xA0)),
            Change::Unchanged
        );
        assert_eq!(
            table.push(&section(0x00, 1, 0, 0, 0, 0xB0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(0x42, 1, 0, 0, 0, 0xC0)),
            Change::Ignored
        );
        assert_eq!(table.push(&section(0x00, 1, 1, 0, 0, 0xD0)), Change::Reset);
    }

    #[test]
    fn another_table_id_ignored() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(0x4F, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Ignored
        );
        // state is not touched
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Unchanged
        );
    }

    #[test]
    fn another_table_id_extension_ignored() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID + 1, 0, 0, 1, 0xB0)),
            Change::Ignored
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Unchanged
        );
    }

    #[test]
    fn version_changed_reset() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 1, 1, 0xA1)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 1, 0, 1, 0xB0)),
            Change::Reset
        );
        // section that caused the reset is stored
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 1, 0, 1, 0xB0)),
            Change::Unchanged
        );
        // sections of the old version are dropped
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 1, 1, 1, 0xA1)),
            Change::Updated
        );
    }

    #[test]
    fn version_wraps_around() {
        // version_number is incremented by 1 modulo 32
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 31, 0, 0, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 0, 0xB0)),
            Change::Reset
        );
    }

    #[test]
    fn last_section_number_changed_reset() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 2, 0xA0)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 2, 2, 0xA2)),
            Change::Updated
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Reset
        );
    }

    #[test]
    fn reserved_bits_do_not_affect_version() {
        let mut table = SectionTable::new();
        let mut s = section(EIT, SERVICE_ID, 5, 0, 0, 0xA0);
        assert_eq!(table.push(&s), Change::Updated);
        s[5] &= !0xC0; // reserved bits cleared
        assert_eq!(table.push(&s), Change::Unchanged);
    }

    #[test]
    fn current_next_indicator_zero_ignored() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );

        let mut next = section(EIT, SERVICE_ID, 1, 0, 1, 0xB0);
        next[5] &= !0x01;
        assert_eq!(table.push(&next), Change::Ignored);
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Unchanged
        );
    }

    #[test]
    fn current_next_indicator_zero_on_empty_ignored() {
        let mut table = SectionTable::new();
        let mut next = section(EIT, SERVICE_ID, 1, 0, 1, 0xB0);
        next[5] &= !0x01;
        assert_eq!(table.push(&next), Change::Ignored);
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
    }

    #[test]
    fn short_form_ignored() {
        // TDT: section_syntax_indicator == 0, 5 bytes of UTC_time
        let mut table = SectionTable::new();
        let tdt = [0x70, 0x70, 0x05, 0xE6, 0x5A, 0x12, 0x34, 0x56];
        assert_eq!(table.push(&tdt), Change::Ignored);
    }

    #[test]
    fn section_number_greater_than_last_ignored() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 2, 1, 0xA0)),
            Change::Ignored
        );
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
    }

    #[test]
    fn short_buffer_ignored() {
        let mut table = SectionTable::new();
        let s = section(EIT, SERVICE_ID, 0, 0, 1, 0xA0);
        for len in 0 .. s.len() {
            assert_eq!(table.push(&s[.. len]), Change::Ignored, "len = {len}");
        }
    }

    #[test]
    fn section_length_too_small_ignored() {
        let mut table = SectionTable::new();
        let mut s = section(EIT, SERVICE_ID, 0, 0, 1, 0xA0);
        s[1] &= 0xF0;
        s[2] = 8;
        assert_eq!(table.push(&s), Change::Ignored);
    }

    #[test]
    fn section_length_too_big_ignored() {
        let mut table = SectionTable::new();
        let mut s = section(EIT, SERVICE_ID, 0, 0, 1, 0xA0);
        s[1] |= 0x0F;
        s[2] = 0xFE; // 4094
        s.resize(3 + 4094, 0);
        assert_eq!(table.push(&s), Change::Ignored);
    }

    #[test]
    fn trailing_bytes_after_section() {
        // stuffing after the section must not be read as CRC_32
        let mut table = SectionTable::new();
        let s = section(EIT, SERVICE_ID, 0, 0, 1, 0xA0);
        let mut stuffed = s.clone();
        stuffed.extend([0xFF; 16]);
        assert_eq!(table.push(&stuffed), Change::Updated);
        assert_eq!(table.push(&s), Change::Unchanged);
    }

    #[test]
    fn max_sections() {
        let mut table = SectionTable::new();
        for n in 0 ..= 255u8 {
            assert_eq!(
                table.push(&section(EIT, SERVICE_ID, 0, n, 255, u32::from(n))),
                Change::Updated
            );
        }
        for n in 0 ..= 255u8 {
            assert_eq!(
                table.push(&section(EIT, SERVICE_ID, 0, n, 255, u32::from(n))),
                Change::Unchanged
            );
        }
    }

    #[test]
    fn clear_resets_everything() {
        let mut table = SectionTable::new();
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        table.clear();
        // same section is new again
        assert_eq!(
            table.push(&section(EIT, SERVICE_ID, 0, 0, 1, 0xA0)),
            Change::Updated
        );
        // after clear any table may be stored
        table.clear();
        assert_eq!(
            table.push(&section(0x4F, 7, 3, 0, 0, 0xB0)),
            Change::Updated
        );
    }
}
