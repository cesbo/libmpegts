/// Program Association Table (PAT) implementation
use crate::psi::{
    Psi,
    PsiDemux,
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
    fn parse(slice: &[u8]) -> Self {
        Self {
            pnr: u16::from_be_bytes([slice[0], slice[1]]),
            pid: u16::from_be_bytes([slice[2], slice[3]]) & 0x1FFF,
        }
    }

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

impl Pat {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 8 + 4 && psi.buffer[0] == 0x00 && psi.check()

        // TODO: check if PSI already parsed
    }

    /// Reads PSI packet and append data into the `Pat`
    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.tsid = u16::from_be_bytes([psi.buffer[3], psi.buffer[4]]);
        self.version = (psi.buffer[5] & 0x3E) >> 1;

        let ptr = &psi.buffer[8 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 4 {
            self.items.push(PatItem::parse(&ptr[skip .. skip + 4]));
            skip += 4;
        }
    }
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

impl From<&Psi> for Pat {
    fn from(psi: &Psi) -> Self {
        let mut pat = Pat::default();
        pat.parse(psi);
        pat
    }
}
