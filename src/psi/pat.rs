/// Program Association Table (PAT) implementation
use crate::{
    pack_bits,
    psi::{
        Psi,
        PsiDemux,
    },
    utils::crc32b,
};

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x0000;

/// Maximum section length without CRC
const PAT_SECTION_SIZE: usize = 1024 - 4;

/// PAT Item
#[derive(Debug, Default)]
pub struct PatItem {
    /// Program Number
    pub pnr: u16,
    /// TS Packet Idetifier
    pub pid: u16,
}

impl PatItem {
    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.pnr.to_be_bytes());
        buffer.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => self.pid
        ));
    }

    #[inline]
    fn size(&self) -> usize {
        4
    }
}

/// Program Association Table provides the correspondence between a `pnr` (Program Number) and
/// the `pid` value of the TS packets which carry the program definition.
#[derive(Default, Debug)]
pub struct Pat {
    /// PAT version
    pub version: u8,
    /// Transport Stream ID to identify actual stream from any other multiplex within a network
    pub tsid: u16,
    /// List of the PAT Items
    pub items: Vec<PatItem>,
}

impl PsiDemux for Pat {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi = Psi::new(0x00);

        psi.buffer.extend_from_slice(&self.tsid.to_be_bytes());
        psi.buffer.extend_from_slice(&pack_bits!(u8,
            reserved: 2 => 0b11,
            version: 5 => self.version,
            current_next_indicator: 1 => 1
        ));
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // section_number and last_section_number

        for item in &self.items {
            if psi.buffer.len() + item.size() > PAT_SECTION_SIZE {
                break;
            }
            item.assemble(&mut psi.buffer);
        }

        vec![psi]
    }
}

#[derive(Debug)]
pub enum PatError {
    InvalidLength,
    InvalidTableId,
    InvalidCrc32,
}

impl core::error::Error for PatError {}

impl std::fmt::Display for PatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatError::InvalidLength => write!(f, "Invalid PAT section length"),
            PatError::InvalidTableId => write!(f, "Invalid PAT table_id"),
            PatError::InvalidCrc32 => write!(f, "Invalid PAT CRC32"),
        }
    }
}

pub struct PatItemRef<'a>(&'a [u8]);

impl<'a> PatItemRef<'a> {
    pub fn pnr(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    pub fn pid(&self) -> u16 {
        u16::from_be_bytes([self.0[2], self.0[3]]) & 0x1FFF
    }
}

impl<'a> TryFrom<&'a [u8]> for PatItemRef<'a> {
    type Error = PatError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            Err(PatError::InvalidLength)
        } else {
            Ok(PatItemRef(&value[0 .. 4]))
        }
    }
}

pub struct PatSectionRef<'a>(&'a [u8]);

impl<'a> PatSectionRef<'a> {
    pub fn tsid(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    pub fn version(&self) -> u8 {
        (self.0[5] & 0x3E) >> 1
    }

    pub fn items(&self) -> impl Iterator<Item = Result<PatItemRef<'a>, PatError>> {
        let ptr = &self.0[8 .. self.0.len() - 4];
        ptr.chunks(4).map(PatItemRef::try_from)
    }

    pub fn crc32(&self) -> u32 {
        let p = &self.0[self.0.len() - 4 ..];
        u32::from_be_bytes([p[0], p[1], p[2], p[3]])
    }
}

impl<'a> TryFrom<&'a [u8]> for PatSectionRef<'a> {
    type Error = PatError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 8 + 4 {
            return Err(PatError::InvalidLength);
        }

        if value[0] != 0x00 {
            return Err(PatError::InvalidTableId);
        }

        let section_length = 3 + (u16::from_be_bytes([value[1], value[2]]) & 0x03FF) as usize;
        if section_length > value.len() {
            return Err(PatError::InvalidLength);
        }

        let pat = PatSectionRef(value);

        let checksum = crc32b(&value[.. value.len() - 4]);
        if checksum != pat.crc32() {
            return Err(PatError::InvalidCrc32);
        }

        Ok(pat)
    }
}
