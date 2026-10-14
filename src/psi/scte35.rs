//! SCTE-35 splice_info_section (ANSI/SCTE 35, section 9), parse only: no
//! caller generates cues, so there is no builder.

use crate::psi::{
    PsiSectionError,
    check_crc32,
    psi_section_length,
};

/// Table ID of the SCTE-35 splice_info_section
pub const SCTE35_TABLE_ID: u8 = 0xfc;

/// table_id through splice_command_type
const SPLICE_HEADER_SIZE: usize = 14;
const SPLICE_LOOP_LENGTH_SIZE: usize = 2;
const SPLICE_CRC_SIZE: usize = 4;
const SPLICE_MIN_SIZE: usize = SPLICE_HEADER_SIZE + SPLICE_LOOP_LENGTH_SIZE + SPLICE_CRC_SIZE;
/// splice_command_length of encoders that leave it unset; the command is
/// walked to find its end
const SPLICE_COMMAND_LENGTH_UNSET: usize = 0xfff;

/// splice_time() with time_specified_flag set, and break_duration()
const SPLICE_TIME_SIZE: usize = 5;
/// unique_program_id, avail_num, avails_expected
const SPLICE_INSERT_TAIL_SIZE: usize = 4;

const SPLICE_NULL: u8 = 0x00;
const SPLICE_INSERT: u8 = 0x05;
const TIME_SIGNAL: u8 = 0x06;
const BANDWIDTH_RESERVATION: u8 = 0x07;
const PRIVATE_COMMAND: u8 = 0xff;

/// Identifier of the descriptors defined by SCTE 35: 'CUEI'
const CUEI: u32 = 0x4355_4549;

/// 33-bit time in 5 bytes, bit 32 in the low bit of the first byte:
/// splice_time(), break_duration() and pts_adjustment.
fn time_33(data: &[u8]) -> u64 {
    (u64::from(data[0] & 0x01) << 32)
        | u64::from(u32::from_be_bytes([data[1], data[2], data[3], data[4]]))
}

/// Length of splice_time(), `None` when `data` is empty
fn splice_time_len(data: &[u8]) -> Option<usize> {
    Some(if data.first()? & 0x80 != 0 {
        SPLICE_TIME_SIZE
    } else {
        1
    })
}

/// Length of splice_insert(), `None` when `data` is too short for it
fn splice_insert_len(data: &[u8]) -> Option<usize> {
    if data.get(4)? & 0x80 != 0 {
        return Some(5);
    }

    let flags = *data.get(5)?;
    let immediate = flags & 0x10 != 0;
    let mut pos = 6;
    if flags & 0x40 != 0 {
        if !immediate {
            pos += splice_time_len(data.get(pos ..)?)?;
        }
    } else {
        let count = *data.get(pos)?;
        pos += 1;
        for _ in 0 .. count {
            pos += 1;
            if !immediate {
                pos += splice_time_len(data.get(pos ..)?)?;
            }
        }
    }
    if flags & 0x20 != 0 {
        pos += SPLICE_TIME_SIZE;
    }
    pos += SPLICE_INSERT_TAIL_SIZE;

    (pos <= data.len()).then_some(pos)
}

/// Length of a segmentation_descriptor payload after the identifier, `None`
/// when `data` is too short for its fixed fields
fn segmentation_len(data: &[u8]) -> Option<usize> {
    if data.get(4)? & 0x80 != 0 {
        return Some(5);
    }

    let flags = *data.get(5)?;
    let mut pos = 6;
    if flags & 0x80 == 0 {
        pos += 1 + 6 * *data.get(pos)? as usize;
    }
    if flags & 0x40 != 0 {
        pos += 5;
    }
    // upid type and length, upid, segmentation_type_id, segment_num,
    // segments_expected
    Some(pos + 2 + *data.get(pos + 1)? as usize + 3)
}

/// splice_time(): presentation time of a splice point.
///
/// SCTE 35 - 9.8.1
#[derive(Debug, Clone, Copy)]
pub struct SpliceTimeRef<'a>(&'a [u8]);

impl<'a> SpliceTimeRef<'a> {
    /// 33-bit pts_time in 90 kHz ticks, before pts_adjustment.
    /// `None` when time_specified_flag is 0.
    pub fn pts_time(&self) -> Option<u64> {
        (self.0[0] & 0x80 != 0).then(|| time_33(self.0))
    }
}

impl<'a> TryFrom<&'a [u8]> for SpliceTimeRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        splice_time_len(value)
            .and_then(|len| value.get(.. len))
            .map(SpliceTimeRef)
            .ok_or(PsiSectionError::InvalidSectionLength)
    }
}

/// break_duration() of a splice_insert.
///
/// SCTE 35 - 9.8.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakDuration {
    /// `true` when the splicer returns to the network at the end of the break
    pub auto_return: bool,
    /// 33-bit duration in 90 kHz ticks
    pub duration: u64,
}

/// splice_insert(): splice event, cancel or out/in of network.
///
/// SCTE 35 - 9.7.3. Flags read `false` and fields `None` when the event is
/// cancelled. Component-mode entries are length-checked and skipped.
#[derive(Debug, Clone, Copy)]
pub struct SpliceInsertRef<'a>(&'a [u8]);

impl<'a> SpliceInsertRef<'a> {
    /// splice_event_id
    pub fn splice_event_id(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }

    /// `true` when a previously sent event is cancelled
    pub fn splice_event_cancel_indicator(&self) -> bool {
        self.0[4] & 0x80 != 0
    }

    fn flags(&self) -> u8 {
        if self.splice_event_cancel_indicator() {
            0
        } else {
            self.0[5]
        }
    }

    /// `true` for an out point (leaving the network), `false` for an in point
    pub fn out_of_network_indicator(&self) -> bool {
        self.flags() & 0x80 != 0
    }

    /// `true` when all components splice together (program splice mode)
    pub fn program_splice_flag(&self) -> bool {
        self.flags() & 0x40 != 0
    }

    /// `true` when a break_duration is present
    pub fn duration_flag(&self) -> bool {
        self.flags() & 0x20 != 0
    }

    /// `true` when the splice happens at the nearest opportunity
    pub fn splice_immediate_flag(&self) -> bool {
        self.flags() & 0x10 != 0
    }

    /// Program splice time; present in program splice mode without
    /// splice_immediate_flag.
    pub fn splice_time(&self) -> Option<SpliceTimeRef<'a>> {
        (self.program_splice_flag() && !self.splice_immediate_flag())
            .then(|| SpliceTimeRef(&self.0[6 ..]))
    }

    /// Number of components in component splice mode, 0 otherwise.
    pub fn component_count(&self) -> u8 {
        if !self.splice_event_cancel_indicator() && self.0[5] & 0x40 == 0 {
            self.0[6]
        } else {
            0
        }
    }

    /// Break duration, present when duration_flag is set.
    pub fn break_duration(&self) -> Option<BreakDuration> {
        self.duration_flag().then(|| {
            let p = &self.0[self.0.len() - SPLICE_INSERT_TAIL_SIZE - SPLICE_TIME_SIZE ..];
            BreakDuration {
                auto_return: p[0] & 0x80 != 0,
                duration: time_33(p),
            }
        })
    }

    fn tail(&self) -> Option<&'a [u8]> {
        (!self.splice_event_cancel_indicator())
            .then(|| &self.0[self.0.len() - SPLICE_INSERT_TAIL_SIZE ..])
    }

    /// unique_program_id
    pub fn unique_program_id(&self) -> Option<u16> {
        self.tail().map(|p| u16::from_be_bytes([p[0], p[1]]))
    }

    /// avail_num
    pub fn avail_num(&self) -> Option<u8> {
        self.tail().map(|p| p[2])
    }

    /// avails_expected
    pub fn avails_expected(&self) -> Option<u8> {
        self.tail().map(|p| p[3])
    }
}

impl<'a> TryFrom<&'a [u8]> for SpliceInsertRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        splice_insert_len(value)
            .map(|len| SpliceInsertRef(&value[.. len]))
            .ok_or(PsiSectionError::InvalidSectionLength)
    }
}

/// Splice command of a [`SpliceInfoSectionRef`].
#[derive(Debug, Clone, Copy)]
pub enum SpliceCommand<'a> {
    /// splice_null (0x00), a heartbeat
    Null,
    /// splice_insert (0x05)
    Insert(SpliceInsertRef<'a>),
    /// time_signal (0x06)
    TimeSignal(SpliceTimeRef<'a>),
    /// bandwidth_reservation (0x07)
    BandwidthReservation,
    /// private_command (0xFF)
    Private { identifier: u32, data: &'a [u8] },
    /// Any other command type, e.g. splice_schedule (0x04); raw bytes
    Other { command_type: u8, data: &'a [u8] },
    /// encrypted_packet is set: the command is ciphertext
    Encrypted,
}

/// splice_descriptor(): tag, length, identifier and private bytes.
///
/// SCTE 35 - 10.2
#[derive(Debug, Clone, Copy)]
pub struct SpliceDescriptorRef<'a>(&'a [u8]);

impl<'a> SpliceDescriptorRef<'a> {
    /// splice_descriptor_tag
    pub fn splice_descriptor_tag(&self) -> u8 {
        self.0[0]
    }

    /// descriptor_length: bytes after this field, identifier included
    pub fn descriptor_length(&self) -> u8 {
        self.0[1]
    }

    /// Owner of the descriptor; 0x43554549 ('CUEI') for SCTE-35 descriptors
    pub fn identifier(&self) -> u32 {
        u32::from_be_bytes([self.0[2], self.0[3], self.0[4], self.0[5]])
    }

    /// Payload bytes after the identifier
    pub fn data(&self) -> &'a [u8] {
        &self.0[6 ..]
    }
}

/// Iterator over the splice descriptor loop.
pub struct SpliceDescriptorIter<'a> {
    data: &'a [u8],
    error: Option<PsiSectionError>,
}

impl<'a> Iterator for SpliceDescriptorIter<'a> {
    type Item = Result<SpliceDescriptorRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(e) = self.error.take() {
            return Some(Err(e));
        }
        if self.data.is_empty() {
            return None;
        }

        let end = match self.data.get(1) {
            Some(&len) if len >= 4 => 2 + len as usize,
            _ => usize::MAX,
        };
        match self.data.get(.. end) {
            Some(desc) => {
                self.data = &self.data[end ..];
                Some(Ok(SpliceDescriptorRef(desc)))
            }
            None => {
                self.data = &[]; // Stop iteration on error
                Some(Err(PsiSectionError::InvalidDescriptorLength))
            }
        }
    }
}

/// segmentation_descriptor (tag `0x02`, identifier 'CUEI'): segmentation
/// event, type and UPID.
///
/// SCTE 35 - 10.3.3. Fields after the cancel indicator are `None` when the
/// event is cancelled; sub_segment fields and restriction flags are not read.
#[derive(Debug, Clone, Copy)]
pub struct SegmentationDescriptorRef<'a>(&'a [u8]);

impl<'a> SegmentationDescriptorRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x02;

    /// segmentation_event_id
    pub fn segmentation_event_id(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }

    /// `true` when a previously sent event is cancelled
    pub fn segmentation_event_cancel_indicator(&self) -> bool {
        self.0[4] & 0x80 != 0
    }

    /// Offset of segmentation_upid_type, after the components and duration
    fn upid_pos(&self) -> Option<usize> {
        if self.segmentation_event_cancel_indicator() {
            return None;
        }
        let flags = self.0[5];
        let mut pos = 6;
        if flags & 0x80 == 0 {
            pos += 1 + 6 * self.0[6] as usize;
        }
        if flags & 0x40 != 0 {
            pos += 5;
        }
        Some(pos)
    }

    /// 40-bit segmentation_duration in 90 kHz ticks, when
    /// segmentation_duration_flag is set
    pub fn segmentation_duration(&self) -> Option<u64> {
        let pos = self.upid_pos()?;
        (self.0[5] & 0x40 != 0).then(|| {
            let p = &self.0[pos - 5 .. pos];
            u64::from_be_bytes([0, 0, 0, p[0], p[1], p[2], p[3], p[4]])
        })
    }

    /// segmentation_upid_type
    pub fn segmentation_upid_type(&self) -> Option<u8> {
        self.upid_pos().map(|pos| self.0[pos])
    }

    /// Raw segmentation_upid bytes, interpreted by segmentation_upid_type
    pub fn segmentation_upid(&self) -> Option<&'a [u8]> {
        self.upid_pos()
            .map(|pos| &self.0[pos + 2 .. pos + 2 + self.0[pos + 1] as usize])
    }

    fn type_pos(&self) -> Option<usize> {
        self.upid_pos()
            .map(|pos| pos + 2 + self.0[pos + 1] as usize)
    }

    /// segmentation_type_id, e.g. 0x34 Provider Placement Opportunity Start
    pub fn segmentation_type_id(&self) -> Option<u8> {
        self.type_pos().map(|pos| self.0[pos])
    }

    /// segment_num
    pub fn segment_num(&self) -> Option<u8> {
        self.type_pos().map(|pos| self.0[pos + 1])
    }

    /// segments_expected
    pub fn segments_expected(&self) -> Option<u8> {
        self.type_pos().map(|pos| self.0[pos + 2])
    }
}

impl<'a> TryFrom<SpliceDescriptorRef<'a>> for SegmentationDescriptorRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: SpliceDescriptorRef<'a>) -> Result<Self, Self::Error> {
        if value.splice_descriptor_tag() != Self::TAG || value.identifier() != CUEI {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }

        let data = value.data();
        match segmentation_len(data) {
            Some(len) if len <= data.len() => Ok(SegmentationDescriptorRef(data)),
            _ => Err(PsiSectionError::InvalidDescriptorLength),
        }
    }
}

/// SCTE-35 splice_info_section: a splice command (ad insertion cue, time
/// signal) and its splice descriptors, carried on a PMT-listed PID of
/// stream_type 0x86. Parse only, there is no builder.
///
/// SCTE 35 - 9.6
pub struct SpliceInfoSectionRef<'a>(&'a [u8]);

impl<'a> SpliceInfoSectionRef<'a> {
    /// protocol_version, 0 in the current standard
    pub fn protocol_version(&self) -> u8 {
        self.0[3]
    }

    /// `true` when the command and the descriptors are encrypted
    pub fn encrypted_packet(&self) -> bool {
        self.0[4] & 0x80 != 0
    }

    /// encryption_algorithm, 6 bits
    pub fn encryption_algorithm(&self) -> u8 {
        (self.0[4] >> 1) & 0x3f
    }

    /// 33-bit pts_adjustment in 90 kHz ticks, added modulo 2^33 to every
    /// pts_time of the section
    pub fn pts_adjustment(&self) -> u64 {
        time_33(&self.0[4 ..])
    }

    /// cw_index: control word of an encrypted section
    pub fn cw_index(&self) -> u8 {
        self.0[9]
    }

    /// 12-bit tier: authorization tier of the message
    pub fn tier(&self) -> u16 {
        u16::from_be_bytes([self.0[10], self.0[11]]) >> 4
    }

    fn splice_command_length(&self) -> usize {
        (u16::from_be_bytes([self.0[11], self.0[12]]) & 0x0fff) as usize
    }

    /// splice_command_type; ciphertext when encrypted_packet is set
    pub fn splice_command_type(&self) -> u8 {
        self.0[13]
    }

    /// Command bytes after splice_command_type.
    /// An unset splice_command_length (0xFFF) is resolved by walking the
    /// command, which works for the null, insert, time_signal and
    /// bandwidth_reservation commands only.
    fn command_data(&self) -> Result<&'a [u8], PsiSectionError> {
        let rest = &self.0[SPLICE_HEADER_SIZE .. self.0.len() - SPLICE_CRC_SIZE];
        let len = match self.splice_command_length() {
            SPLICE_COMMAND_LENGTH_UNSET => match self.splice_command_type() {
                SPLICE_NULL | BANDWIDTH_RESERVATION => Some(0),
                SPLICE_INSERT => splice_insert_len(rest),
                TIME_SIGNAL => splice_time_len(rest),
                _ => None,
            },
            len => Some(len),
        };
        len.and_then(|len| rest.get(.. len))
            .ok_or(PsiSectionError::InvalidSectionLength)
    }

    /// The splice command; [`SpliceCommand::Encrypted`] when encrypted_packet
    /// is set. Errors when the command overruns the section.
    pub fn splice_command(&self) -> Result<SpliceCommand<'a>, PsiSectionError> {
        if self.encrypted_packet() {
            return Ok(SpliceCommand::Encrypted);
        }

        let data = self.command_data()?;
        Ok(match self.splice_command_type() {
            SPLICE_NULL => SpliceCommand::Null,
            SPLICE_INSERT => SpliceCommand::Insert(SpliceInsertRef::try_from(data)?),
            TIME_SIGNAL => SpliceCommand::TimeSignal(SpliceTimeRef::try_from(data)?),
            BANDWIDTH_RESERVATION => SpliceCommand::BandwidthReservation,
            PRIVATE_COMMAND => {
                if data.len() < 4 {
                    return Err(PsiSectionError::InvalidSectionLength);
                }
                SpliceCommand::Private {
                    identifier: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
                    data: &data[4 ..],
                }
            }
            command_type => SpliceCommand::Other { command_type, data },
        })
    }

    fn descriptor_loop(&self) -> Result<&'a [u8], PsiSectionError> {
        let start = SPLICE_HEADER_SIZE + self.command_data()?.len();
        let body = &self.0[.. self.0.len() - SPLICE_CRC_SIZE];
        let p = body
            .get(start .. start + SPLICE_LOOP_LENGTH_SIZE)
            .ok_or(PsiSectionError::InvalidSectionLength)?;
        let start = start + SPLICE_LOOP_LENGTH_SIZE;
        // alignment_stuffing may follow the loop
        body.get(start .. start + u16::from_be_bytes([p[0], p[1]]) as usize)
            .ok_or(PsiSectionError::InvalidSectionLength)
    }

    /// Splice descriptors; empty when encrypted_packet is set. A command or
    /// a descriptor loop overrunning the section yields one error.
    pub fn descriptors(&self) -> SpliceDescriptorIter<'a> {
        let (data, error) = if self.encrypted_packet() {
            (&[][..], None)
        } else {
            match self.descriptor_loop() {
                Ok(data) => (data, None),
                Err(e) => (&[][..], Some(e)),
            }
        };
        SpliceDescriptorIter { data, error }
    }
}

impl<'a> TryFrom<&'a [u8]> for SpliceInfoSectionRef<'a> {
    type Error = PsiSectionError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < SPLICE_MIN_SIZE {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if value[0] != SCTE35_TABLE_ID {
            return Err(PsiSectionError::InvalidTableId);
        }

        let section_length = psi_section_length(value);
        if section_length < SPLICE_MIN_SIZE || section_length > value.len() {
            return Err(PsiSectionError::InvalidSectionLength);
        }

        if !check_crc32(&value[.. section_length]) {
            return Err(PsiSectionError::InvalidCrc32);
        }

        Ok(SpliceInfoSectionRef(&value[.. section_length]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::crc32b;

    // SCTE 35 section 14 samples, base64 decoded; CRC_32 checked by try_from

    // 14.1 time_signal - Placement Opportunity Start
    const SAMPLE_14_1: &[u8] = &[
        0xfc, 0x30, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xf0, 0x05, 0x06, 0xfe,
        0x72, 0xbd, 0x00, 0x50, 0x00, 0x1e, 0x02, 0x1c, 0x43, 0x55, 0x45, 0x49, 0x48, 0x00, 0x00,
        0x8e, 0x7f, 0xcf, 0x00, 0x01, 0xa5, 0x99, 0xb0, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x2c,
        0xa0, 0xa1, 0x8a, 0x34, 0x02, 0x00, 0x9a, 0xc9, 0xd1, 0x7e,
    ];

    // 14.2 splice_insert
    const SAMPLE_14_2: &[u8] = &[
        0xfc, 0x30, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xf0, 0x14, 0x05, 0x48,
        0x00, 0x00, 0x8f, 0x7f, 0xef, 0xfe, 0x73, 0x69, 0xc0, 0x2e, 0xfe, 0x00, 0x52, 0xcc, 0xf5,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x08, 0x43, 0x55, 0x45, 0x49, 0x00, 0x00, 0x01,
        0x35, 0x62, 0xdb, 0xa3, 0x0a,
    ];

    // 14.3 time_signal - Placement Opportunity End
    const SAMPLE_14_3: &[u8] = &[
        0xfc, 0x30, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xf0, 0x05, 0x06, 0xfe,
        0x74, 0x62, 0x90, 0xa0, 0x00, 0x19, 0x02, 0x17, 0x43, 0x55, 0x45, 0x49, 0x48, 0x00, 0x00,
        0x8e, 0x7f, 0x9f, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00, 0x2c, 0xa0, 0xa1, 0x8a, 0x35, 0x02,
        0x00, 0xa9, 0xcc, 0x67, 0x58,
    ];

    // splice_insert with splice_command_length 0xFFF and 3 bytes of
    // alignment_stuffing (Comcast scte35-go test vector)
    const LEGACY_STUFFED: &[u8] = &[
        0xfc, 0x30, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x05, 0x62,
        0x00, 0x0f, 0xe7, 0x7f, 0x5f, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x90, 0x9b, 0xfe,
        0x60, 0xfb, 0x44,
    ];

    // splice_null heartbeat, the smallest section
    const SPLICE_NULL_SAMPLE: &[u8] = &[
        0xfc, 0x30, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xf0, 0x00, 0x00, 0x00,
        0x00, 0x7a, 0x4f, 0xbf, 0xff,
    ];

    /// Replaces the CRC_32 of `section` after an edit
    fn resealed(section: &[u8], edit: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
        let mut v = section[.. section.len() - SPLICE_CRC_SIZE].to_vec();
        edit(&mut v);
        let len = (v.len() + SPLICE_CRC_SIZE - 3) as u16;
        v[1] = (v[1] & 0xf0) | (len >> 8) as u8;
        v[2] = len as u8;
        let crc = crc32b(&v);
        v.extend_from_slice(&crc.to_be_bytes());
        v
    }

    fn segmentation<'a>(section: &SpliceInfoSectionRef<'a>) -> SegmentationDescriptorRef<'a> {
        let mut descriptors = section.descriptors();
        let desc = descriptors.next().unwrap().unwrap();
        assert!(descriptors.next().is_none());
        SegmentationDescriptorRef::try_from(desc).unwrap()
    }

    #[test]
    fn parses_placement_opportunity_start() {
        let section = SpliceInfoSectionRef::try_from(SAMPLE_14_1).unwrap();
        assert_eq!(section.protocol_version(), 0);
        assert!(!section.encrypted_packet());
        assert_eq!(section.encryption_algorithm(), 0);
        assert_eq!(section.pts_adjustment(), 0);
        assert_eq!(section.cw_index(), 0xff);
        assert_eq!(section.tier(), 0x0fff);
        assert_eq!(section.splice_command_type(), 0x06);

        let Ok(SpliceCommand::TimeSignal(time)) = section.splice_command() else {
            panic!("time_signal expected");
        };
        assert_eq!(time.pts_time(), Some(0x0_72bd_0050));

        let seg = segmentation(&section);
        assert_eq!(seg.segmentation_event_id(), 0x4800_008e);
        assert!(!seg.segmentation_event_cancel_indicator());
        // 10.3.3: program_segmentation_flag 1, so no component loop before
        // the 40-bit duration
        assert_eq!(seg.segmentation_duration(), Some(0x00_01a5_99b0));
        assert_eq!(seg.segmentation_upid_type(), Some(0x08));
        assert_eq!(
            seg.segmentation_upid(),
            Some(&[0x00, 0x00, 0x00, 0x00, 0x2c, 0xa0, 0xa1, 0x8a][..])
        );
        assert_eq!(seg.segmentation_type_id(), Some(0x34));
        assert_eq!(seg.segment_num(), Some(2));
        assert_eq!(seg.segments_expected(), Some(0));
    }

    #[test]
    fn parses_splice_insert() {
        let section = SpliceInfoSectionRef::try_from(SAMPLE_14_2).unwrap();
        let Ok(SpliceCommand::Insert(insert)) = section.splice_command() else {
            panic!("splice_insert expected");
        };
        assert_eq!(insert.splice_event_id(), 0x4800_008f);
        assert!(!insert.splice_event_cancel_indicator());
        assert!(insert.out_of_network_indicator());
        assert!(insert.program_splice_flag());
        assert!(insert.duration_flag());
        assert!(!insert.splice_immediate_flag());
        assert_eq!(
            insert.splice_time().unwrap().pts_time(),
            Some(0x0_7369_c02e)
        );
        assert_eq!(insert.component_count(), 0);
        assert_eq!(
            insert.break_duration(),
            Some(BreakDuration {
                auto_return: true,
                duration: 0x0_0052_ccf5,
            })
        );
        assert_eq!(insert.unique_program_id(), Some(0));
        assert_eq!(insert.avail_num(), Some(0));
        assert_eq!(insert.avails_expected(), Some(0));

        // 10.3.1 avail_descriptor: provider_avail_id 0x135
        let mut descriptors = section.descriptors();
        let desc = descriptors.next().unwrap().unwrap();
        assert!(descriptors.next().is_none());
        assert_eq!(desc.splice_descriptor_tag(), 0x00);
        assert_eq!(desc.descriptor_length(), 8);
        assert_eq!(desc.identifier(), CUEI);
        assert_eq!(desc.data(), [0x00, 0x00, 0x01, 0x35]);
        assert!(matches!(
            SegmentationDescriptorRef::try_from(desc),
            Err(PsiSectionError::InvalidDescriptorTag)
        ));
    }

    #[test]
    fn parses_placement_opportunity_end() {
        let section = SpliceInfoSectionRef::try_from(SAMPLE_14_3).unwrap();
        let Ok(SpliceCommand::TimeSignal(time)) = section.splice_command() else {
            panic!("time_signal expected");
        };
        assert_eq!(time.pts_time(), Some(0x0_7462_90a0));

        let seg = segmentation(&section);
        assert_eq!(seg.segmentation_event_id(), 0x4800_008e);
        assert_eq!(seg.segmentation_duration(), None);
        assert_eq!(seg.segmentation_upid_type(), Some(0x08));
        assert_eq!(seg.segmentation_type_id(), Some(0x35));
        assert_eq!(seg.segment_num(), Some(2));
        assert_eq!(seg.segments_expected(), Some(0));
    }

    #[test]
    fn parses_unset_command_length() {
        let section = SpliceInfoSectionRef::try_from(LEGACY_STUFFED).unwrap();
        let Ok(SpliceCommand::Insert(insert)) = section.splice_command() else {
            panic!("splice_insert expected");
        };
        assert_eq!(insert.splice_event_id(), 0x6200_0fe7);
        assert!(insert.splice_immediate_flag());
        assert!(insert.splice_time().is_none());
        assert!(insert.break_duration().is_none());
        assert_eq!(insert.unique_program_id(), Some(9));
        // empty loop, then alignment_stuffing up to CRC_32
        assert!(section.descriptors().next().is_none());
    }

    #[test]
    fn parses_splice_null() {
        let section = SpliceInfoSectionRef::try_from(SPLICE_NULL_SAMPLE).unwrap();
        assert!(matches!(section.splice_command(), Ok(SpliceCommand::Null)));
        assert!(section.descriptors().next().is_none());
    }

    #[test]
    fn parses_header_bits() {
        // encryption_algorithm and pts_adjustment bit 32 share byte 4
        let data = resealed(SPLICE_NULL_SAMPLE, |v| {
            v[4] = 0x7f;
            v[5 .. 9].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
            v[9] = 0x5a;
            v[10] = 0xab;
            v[11] = 0xc0;
        });
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(!section.encrypted_packet());
        assert_eq!(section.encryption_algorithm(), 0x3f);
        assert_eq!(section.pts_adjustment(), 0x1_1234_5678);
        assert_eq!(section.cw_index(), 0x5a);
        assert_eq!(section.tier(), 0xabc);
        assert!(matches!(section.splice_command(), Ok(SpliceCommand::Null)));
    }

    #[test]
    fn parses_private_command() {
        // 9.7.6: identifier, then private bytes
        let data = resealed(SPLICE_NULL_SAMPLE, |v| {
            v[12] = 6;
            v[13] = PRIVATE_COMMAND;
            v.extend_from_slice(&[0x43, 0x55, 0x45, 0x49, 0xaa, 0xbb, 0x00, 0x00]);
            v.drain(14 .. 16);
        });
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        let Ok(SpliceCommand::Private {
            identifier,
            data: bytes,
        }) = section.splice_command()
        else {
            panic!("private_command expected");
        };
        assert_eq!(identifier, CUEI);
        assert_eq!(bytes, [0xaa, 0xbb]);
        assert!(section.descriptors().next().is_none());

        // identifier cut short
        let data = resealed(&data, |v| {
            v[12] = 2;
            v.drain(16 .. 20);
        });
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(matches!(
            section.splice_command(),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn parses_cancelled_insert() {
        let cancel = [0x05, 0x00, 0x00, 0x00, 0x01, 0xff];
        let data = resealed(SPLICE_NULL_SAMPLE, |v| {
            v[12] = 5;
            v.truncate(13);
            v.extend_from_slice(&cancel);
            v.extend_from_slice(&[0x00, 0x00]);
        });
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        let Ok(SpliceCommand::Insert(insert)) = section.splice_command() else {
            panic!("splice_insert expected");
        };
        assert_eq!(insert.splice_event_id(), 1);
        assert!(insert.splice_event_cancel_indicator());
        assert!(!insert.out_of_network_indicator());
        assert!(insert.splice_time().is_none());
        assert!(insert.unique_program_id().is_none());
    }

    #[test]
    fn parses_component_insert() {
        // 9.7.3 component mode: component_tag + splice_time() of 5 or 1 bytes
        #[rustfmt::skip]
        let command = [
            0x05, 0x00, 0x00, 0x00, 0x02, 0x7f, 0x8f, 0x02,
            0x01, 0xfe, 0x00, 0x00, 0x00, 0x10,
            0x02, 0x7f,
            0x00, 0x07, 0x01, 0x02,
        ];
        let data = resealed(SPLICE_NULL_SAMPLE, |v| {
            v[12] = (command.len() - 1) as u8;
            v.truncate(13);
            v.extend_from_slice(&command);
            v.extend_from_slice(&[0x00, 0x00]);
        });
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        let Ok(SpliceCommand::Insert(insert)) = section.splice_command() else {
            panic!("splice_insert expected");
        };
        assert!(insert.out_of_network_indicator());
        assert!(!insert.program_splice_flag());
        assert_eq!(insert.component_count(), 2);
        assert!(insert.splice_time().is_none());
        assert_eq!(insert.unique_program_id(), Some(7));
        assert_eq!(insert.avail_num(), Some(1));
        assert_eq!(insert.avails_expected(), Some(2));

        // a third component runs past the command
        let data = resealed(&data, |v| v[20] = 3);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(matches!(
            section.splice_command(),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn rejects_wrong_table_id() {
        let mut data = SAMPLE_14_1.to_vec();
        data[0] = 0xfd;
        assert!(matches!(
            SpliceInfoSectionRef::try_from(&data[..]),
            Err(PsiSectionError::InvalidTableId)
        ));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut data = SAMPLE_14_1.to_vec();
        *data.last_mut().unwrap() ^= 0x01;
        assert!(matches!(
            SpliceInfoSectionRef::try_from(&data[..]),
            Err(PsiSectionError::InvalidCrc32)
        ));
    }

    #[test]
    fn rejects_short_slice() {
        assert!(matches!(
            SpliceInfoSectionRef::try_from(&SAMPLE_14_1[.. 40]),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn rejects_truncated_command() {
        // time_signal declaring 5 bytes with 2 left before the CRC_32
        let data = resealed(SAMPLE_14_1, |v| v.truncate(16));
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(matches!(
            section.splice_command(),
            Err(PsiSectionError::InvalidSectionLength)
        ));
        let mut descriptors = section.descriptors();
        assert!(matches!(
            descriptors.next(),
            Some(Err(PsiSectionError::InvalidSectionLength))
        ));
        assert!(descriptors.next().is_none());

        // splice_insert declaring 16 of its 20 bytes
        let data = resealed(SAMPLE_14_2, |v| v[12] = 0x10);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(matches!(
            section.splice_command(),
            Err(PsiSectionError::InvalidSectionLength)
        ));
    }

    #[test]
    fn rejects_descriptor_overrun() {
        // descriptor_loop_length past CRC_32
        let data = resealed(SAMPLE_14_3, |v| v[20] = 0x30);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(section.splice_command().is_ok());
        let mut descriptors = section.descriptors();
        assert!(matches!(
            descriptors.next(),
            Some(Err(PsiSectionError::InvalidSectionLength))
        ));
        assert!(descriptors.next().is_none());

        // descriptor_length past the loop
        let data = resealed(SAMPLE_14_2, |v| v[37] = 9);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        let mut descriptors = section.descriptors();
        assert!(matches!(
            descriptors.next(),
            Some(Err(PsiSectionError::InvalidDescriptorLength))
        ));
        assert!(descriptors.next().is_none());

        // descriptor_length too short for the identifier
        let data = resealed(SAMPLE_14_2, |v| v[37] = 3);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(matches!(
            section.descriptors().next(),
            Some(Err(PsiSectionError::InvalidDescriptorLength))
        ));
    }

    #[test]
    fn rejects_truncated_segmentation() {
        let section = SpliceInfoSectionRef::try_from(SAMPLE_14_1).unwrap();
        let desc = section.descriptors().next().unwrap().unwrap();
        // cut inside segmentation_upid
        let desc = SpliceDescriptorRef(&desc.0[.. 24]);
        assert!(matches!(
            SegmentationDescriptorRef::try_from(desc),
            Err(PsiSectionError::InvalidDescriptorLength)
        ));
    }

    #[test]
    fn hides_encrypted_payload() {
        // 9.6.1: encrypted_packet is bit 7 of byte 4
        let data = resealed(SAMPLE_14_1, |v| v[4] |= 0x80);
        let section = SpliceInfoSectionRef::try_from(&data[..]).unwrap();
        assert!(section.encrypted_packet());
        assert!(matches!(
            section.splice_command(),
            Ok(SpliceCommand::Encrypted)
        ));
        assert!(section.descriptors().next().is_none());
    }
}
