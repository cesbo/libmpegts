use crate::{
    psi::{
        DescriptorsRef,
        PsiSectionError,
        PsiSectionMut,
        check_crc32,
        psi_section_length,
    },
    utils::{
        BcdTime,
        MjdFrom,
    },
};

pub const EIT_PID: u16 = 0x0012;

/// Table ID of EIT present/following for the actual transport stream
pub const EIT_TABLE_ID_PF_ACTUAL: u8 = 0x4e;
/// Table ID of EIT present/following for another transport stream
pub const EIT_TABLE_ID_PF_OTHER: u8 = 0x4f;

const EIT_HEADER_SIZE: usize = 14;
const EIT_ITEM_HEADER_SIZE: usize = 12;
const EIT_CRC_SIZE: usize = 4;

pub struct EitEventRef<'a>(&'a [u8]);

impl<'a> EitEventRef<'a> {
    /// Event identification number
    pub fn event_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// Event start time in UTC
    pub fn start_time(&self) -> u64 {
        u64::from_mjd([self.0[2], self.0[3]])
            + u32::from_bcd_time([self.0[4], self.0[5], self.0[6]]) as u64
    }

    /// Event duration in seconds
    pub fn duration(&self) -> u32 {
        u32::from_bcd_time([self.0[7], self.0[8], self.0[9]])
    }

    /// Indicating the status of the event
    /// * `0` - undefined
    /// * `1` - not running
    /// * `2` - starts in a few seconds (e.g. for video recording)
    /// * `3` - pausing
    /// * `4` - running
    /// * `5` - service off-air
    pub fn running_status(&self) -> u8 {
        (self.0[10] & 0xe0) >> 5
    }

    /// On `true` indicates that access is controlled by a CA system
    pub fn free_ca_mode(&self) -> bool {
        (self.0[10] & 0x10) != 0
    }

    /// Program element descriptors
    pub fn event_descriptors(&self) -> Option<DescriptorsRef<'_>> {
        (self.0.len() > EIT_ITEM_HEADER_SIZE).then(|| self.0[EIT_ITEM_HEADER_SIZE ..].into())
    }

    /// Returns full item length including descriptors
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a [u8]> for EitEventRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < EIT_ITEM_HEADER_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }
        let desc_length = (u16::from_be_bytes([value[10], value[11]]) & 0x0fff) as usize;
        let item_length = EIT_ITEM_HEADER_SIZE + desc_length;
        if value.len() >= item_length {
            Ok(EitEventRef(&value[.. item_length]))
        } else {
            Err(PsiSectionError::InvalidSectionLength)
        }
    }
}

pub struct EitEventIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for EitEventIter<'a> {
    type Item = Result<EitEventRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let remaining = &self.data[self.offset ..];
        match EitEventRef::try_from(remaining) {
            Ok(item) => {
                self.offset += item.len();
                Some(Ok(item))
            }
            Err(e) => {
                self.offset = self.data.len(); // Stop iteration on error
                Some(Err(e))
            }
        }
    }
}

/// Event Information Table provides information in chronological order
/// regarding the events contained within each service.
pub struct EitSectionRef<'a>(&'a [u8]);

impl<'a> EitSectionRef<'a> {
    /// Table ID
    /// * `0x4e` - actual TS, present/following event information
    /// * `0x4f` - other TS, present/following event information
    /// * `0x50 ..= 0x5f` - actual TS, event schedule information
    /// * `0x60 ..= 0x6f` - other TS, event schedule information
    pub fn table_id(&self) -> u8 {
        self.0[0]
    }

    /// EIT version.
    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3e) >> 1
    }

    /// `true` when the section carries the currently applicable information,
    /// `false` when it describes the next (future) version.
    pub fn current_next_indicator(&self) -> bool {
        (self.0[5] & 0x01) != 0
    }

    /// Number of this section within the sub-table.
    pub fn section_number(&self) -> u8 {
        self.0[6]
    }

    /// Number of the last section of this sub-table.
    pub fn last_section_number(&self) -> u8 {
        self.0[7]
    }

    /// Program number
    pub fn service_id(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    /// Transport Stream Identifier
    pub fn transport_stream_id(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]])
    }

    /// Original Network ID
    pub fn original_network_id(&self) -> u16 {
        u16::from_be_bytes([self.0[10], self.0[11]])
    }

    /// Number of the last section of this segment of the sub-table.
    pub fn segment_last_section_number(&self) -> u8 {
        self.0[12]
    }

    /// Last table ID used for the event information of this service.
    pub fn last_table_id(&self) -> u8 {
        self.0[13]
    }

    /// Iterator for EIT events
    pub fn events(&self) -> EitEventIter<'a> {
        let items_start = EIT_HEADER_SIZE;
        let items_end = self.0.len() - EIT_CRC_SIZE;
        EitEventIter {
            data: &self.0[items_start .. items_end],
            offset: 0,
        }
    }

    /// CRC32 checksum
    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - EIT_CRC_SIZE ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for EitSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < EIT_HEADER_SIZE + EIT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            0x4e ..= 0x6f => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        let section_length = psi_section_length(value);
        if section_length < EIT_HEADER_SIZE + EIT_CRC_SIZE || section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(EitSectionRef(&value[.. section_length]))
    }
}

/// Maps an EIT table ID into the actual or the other family, preserving
/// present/following vs schedule and the schedule part index.
fn eit_table_id_family(table_id: u8, actual: bool) -> u8 {
    match (table_id, actual) {
        (EIT_TABLE_ID_PF_OTHER, true) => EIT_TABLE_ID_PF_ACTUAL,
        (EIT_TABLE_ID_PF_ACTUAL, false) => EIT_TABLE_ID_PF_OTHER,
        (id @ 0x60 ..= 0x6f, true) => id - 0x10,
        (id @ 0x50 ..= 0x5f, false) => id + 0x10,
        (id, _) => id,
    }
}

/// In-place patcher for one complete EIT section.
///
/// # Examples
///
/// ```
/// use libmpegts::psi::{EitSectionMut, EitSectionRef};
///
/// let mut section = vec![
///     0x4e, 0xf0, 0x0f, // present/following, section_length 15
///     0x00, 0x01, // service_id
///     0xc1, 0x00, 0x00, // version 0, section 0 of 0
///     0x00, 0x01, // transport_stream_id
///     0x00, 0x55, // original_network_id
///     0x00, 0x4e, // segment_last_section_number, last_table_id
///     0x00, 0x00, 0x00, 0x00, // CRC32, computed below
/// ];
///
/// let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
/// eit.set_actual(false);
/// eit.set_transport_stream_id(7);
/// eit.set_original_network_id(1);
/// eit.update_crc32();
///
/// let eit = EitSectionRef::try_from(&section[..]).unwrap();
/// assert_eq!(eit.table_id(), 0x4f);
/// assert_eq!(eit.last_table_id(), 0x4f);
/// assert_eq!(eit.transport_stream_id(), 7);
/// assert_eq!(eit.original_network_id(), 1);
/// ```
pub struct EitSectionMut<'a>(PsiSectionMut<'a>);

impl<'a> EitSectionMut<'a> {
    /// Moves the section between the actual and the other table ID family,
    /// patching `table_id` and `last_table_id` together.
    pub fn set_actual(&mut self, actual: bool) {
        let section = self.0.as_mut();
        section[0] = eit_table_id_family(section[0], actual);
        section[13] = eit_table_id_family(section[13], actual);
    }

    /// Sets the service ID.
    pub fn set_service_id(&mut self, service_id: u16) {
        self.0.as_mut()[3 .. 5].copy_from_slice(&service_id.to_be_bytes());
    }

    /// Sets the transport stream ID.
    pub fn set_transport_stream_id(&mut self, transport_stream_id: u16) {
        self.0.as_mut()[8 .. 10].copy_from_slice(&transport_stream_id.to_be_bytes());
    }

    /// Sets the original network ID.
    pub fn set_original_network_id(&mut self, original_network_id: u16) {
        self.0.as_mut()[10 .. 12].copy_from_slice(&original_network_id.to_be_bytes());
    }

    /// Sets the 5-bit `version_number`, preserving the reserved bits and
    /// `current_next_indicator`.
    pub fn set_version(&mut self, version: u8) {
        self.0.set_version(version);
    }

    /// Recomputes the CRC32 over the section body and writes it into the
    /// last four bytes.
    pub fn update_crc32(&mut self) {
        self.0.update_crc32();
    }
}

impl<'a> TryFrom<&'a mut [u8]> for EitSectionMut<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a mut [u8]) -> Result<Self, Self::Error> {
        if value.len() < EIT_HEADER_SIZE + EIT_CRC_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        match value[0] {
            0x4e ..= 0x6f => (),
            _ => return Err(PsiSectionError::InvalidTableId),
        };

        Ok(Self(PsiSectionMut::try_from(value)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::crc32b;

    fn eit_section(table_id: u8, last_table_id: u8) -> Vec<u8> {
        let mut section = vec![
            table_id, 0xf0, 0x00, // section_length patched below
            0x00, 0x01, // service_id
            0xc1, 0x00, 0x00, // version 0, section 0 of 0
            0x00, 0x02, // transport_stream_id
            0x00, 0x55, // original_network_id
            0x00, // segment_last_section_number
            last_table_id,
        ];
        // One event without descriptors
        section.extend_from_slice(&[
            0x00, 0x01, // event_id
            0xef, 0xec, 0x08, 0x00, 0x00, // start_time
            0x00, 0x30, 0x00, // duration 00:30:00
            0x80, 0x00, // running_status 4, free_ca_mode 0
        ]);

        let section_length = (section.len() + EIT_CRC_SIZE - 3) as u16;
        section[1] = 0xf0 | ((section_length >> 8) as u8 & 0x0f);
        section[2] = section_length as u8;

        let crc = crc32b(&section);
        section.extend_from_slice(&crc.to_be_bytes());
        section
    }

    #[test]
    fn patches_ids() {
        let mut section = eit_section(0x4e, 0x4e);

        let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
        eit.set_service_id(0x1234);
        eit.set_transport_stream_id(7);
        eit.set_original_network_id(0x2000);
        eit.set_version(9);
        eit.update_crc32();

        let eit = EitSectionRef::try_from(&section[..]).unwrap();
        assert_eq!(eit.table_id(), 0x4e);
        assert_eq!(eit.service_id(), 0x1234);
        assert_eq!(eit.transport_stream_id(), 7);
        assert_eq!(eit.original_network_id(), 0x2000);
        assert_eq!(eit.version(), 9);

        let event = eit.events().next().unwrap().unwrap();
        assert_eq!(event.event_id(), 1);
        assert_eq!(event.start_time(), 1_800_000_000);
        assert_eq!(event.duration(), 30 * 60);
    }

    #[test]
    fn flips_pf_family() {
        let mut section = eit_section(0x4e, 0x4e);

        let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
        eit.set_actual(false);
        eit.update_crc32();

        let parsed = EitSectionRef::try_from(&section[..]).unwrap();
        assert_eq!(parsed.table_id(), 0x4f);
        assert_eq!(parsed.last_table_id(), 0x4f);

        let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
        eit.set_actual(true);
        eit.update_crc32();

        let parsed = EitSectionRef::try_from(&section[..]).unwrap();
        assert_eq!(parsed.table_id(), 0x4e);
        assert_eq!(parsed.last_table_id(), 0x4e);
    }

    #[test]
    fn flips_schedule_family_preserving_part() {
        let mut section = eit_section(0x52, 0x53);

        let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
        eit.set_actual(false);
        eit.update_crc32();

        let parsed = EitSectionRef::try_from(&section[..]).unwrap();
        assert_eq!(parsed.table_id(), 0x62);
        assert_eq!(parsed.last_table_id(), 0x63);
    }

    #[test]
    fn set_actual_is_idempotent() {
        let mut section = eit_section(0x4f, 0x4f);

        let mut eit = EitSectionMut::try_from(&mut section[..]).unwrap();
        eit.set_actual(false);
        eit.update_crc32();

        let parsed = EitSectionRef::try_from(&section[..]).unwrap();
        assert_eq!(parsed.table_id(), 0x4f);
        assert_eq!(parsed.last_table_id(), 0x4f);
    }

    #[test]
    fn rejects_wrong_table_id() {
        let mut section = eit_section(0x4e, 0x4e);
        section[0] = 0x42;
        assert!(EitSectionMut::try_from(&mut section[..]).is_err());
    }

    #[test]
    fn rejects_truncated_section() {
        let mut section = eit_section(0x4e, 0x4e);
        section.truncate(16);
        assert!(EitSectionMut::try_from(&mut section[..]).is_err());
    }
}
