use crate::psi::{
    Descriptors,
    Psi,
    PsiDemux,
};

pub const NIT_PID: u16 = 0x0010;

/// Maximum section length without CRC
const NIT_SECTION_SIZE: usize = 1024 - 4;

/// NIT Item.
#[derive(Debug, Default)]
pub struct NitItem {
    /// Identifier which serves as a label for identification of this
    /// TS from any other multiplex within the delivery system.
    pub tsid: u16,
    /// Label identifying the network_id of the originating delivery system.
    pub onid: u16,
    /// List of descriptors.
    pub descriptors: Descriptors,
}

impl NitItem {
    pub fn parse(slice: &[u8]) -> Self {
        let mut item = Self {
            tsid: u16::from_be_bytes([slice[0], slice[1]]),
            onid: u16::from_be_bytes([slice[2], slice[3]]),
            ..Default::default()
        };

        item.descriptors.parse(&slice[6 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.tsid.to_be_bytes());
        buffer.extend_from_slice(&self.onid.to_be_bytes());

        let skip = buffer.len();
        buffer.extend_from_slice(&[0xF0, 0x00]); // placeholder for descriptors length
        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        let flags_and_len = 0xF000 | descriptors_len;
        buffer[skip .. skip + 2].copy_from_slice(&flags_and_len.to_be_bytes());
    }

    #[inline]
    fn size(&self) -> usize {
        6 + self.descriptors.size()
    }
}

/// The NIT conveys information relating to the physical organization
/// of the multiplexes/TSs carried via a given network,
/// and the characteristics of the network itself.
///
/// EN 300 468 - 5.2.1
#[derive(Debug, Default)]
pub struct Nit {
    /// Identifies to which table the section belongs:
    /// * `0x40` - actual network
    /// * `0x41` - other network
    pub table_id: u8,
    /// NIT version.
    pub version: u8,
    /// Identifier which serves as a label the delivery system,
    /// about which the NIT informs, from any other delivery system.
    pub network_id: u16,
    /// List of descriptors.
    pub descriptors: Descriptors,
    /// List of NIT items.
    pub items: Vec<NitItem>,
}

impl Nit {
    #[inline]
    pub fn check(&self, psi: &Psi) -> bool {
        psi.size >= 12 + 4 &&
        (psi.buffer[0] & 0xFE) == 0x40 && /* 0x40 or 0x41 */
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if !self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.network_id = u16::from_be_bytes([psi.buffer[3], psi.buffer[4]]);
        self.version = (psi.buffer[5] & 0x3E) >> 1;

        let descriptors_len =
            (u16::from_be_bytes([psi.buffer[8], psi.buffer[9]]) & 0x0FFF) as usize;
        self.descriptors
            .parse(&psi.buffer[10 .. 10 + descriptors_len]);

        let ptr = &psi.buffer[12 + descriptors_len .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 6 {
            let item_len =
                6 + (u16::from_be_bytes([ptr[skip + 4], ptr[skip + 5]]) & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items
                .push(NitItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self, first: bool) -> Psi {
        let mut psi = Psi::new(self.table_id, 3, self.version);
        psi.buffer[1] = 0xF0; // set reserved_future_use bit
        psi.buffer.extend_from_slice(&self.network_id.to_be_bytes());
        psi.buffer
            .push(set_bits!(8, 0b11, 2, self.version, 5, 1, 1));
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder for section_number and last_section_number

        let skip = psi.buffer.len();
        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder for descriptors_length
        let desc_len = if first {
            self.descriptors.assemble(&mut psi.buffer) as u16
        } else {
            0
        };
        psi.buffer[skip .. skip + 2].copy_from_slice(&(0xF000 | desc_len).to_be_bytes());

        psi.buffer.extend_from_slice(&[0x00, 0x00]); // placeholder for transport_stream_loop_length

        psi
    }
}

impl PsiDemux for Nit {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init(true)];

        for item in &self.items {
            {
                let psi = psi_list.last_mut().unwrap();
                if NIT_SECTION_SIZE >= psi.buffer.len() + item.size() {
                    item.assemble(&mut psi.buffer);
                    continue;
                }
            }

            let mut psi = self.psi_init(false);
            item.assemble(&mut psi.buffer);
            psi_list.push(psi);
        }

        for item in &mut psi_list {
            let descriptors_len =
                (u16::from_be_bytes([item.buffer[8], item.buffer[9]]) & 0x0FFF) as usize;
            let items_len = (item.buffer.len() - 12 - descriptors_len) as u16;
            let skip = 10 + descriptors_len;
            item.buffer[skip .. skip + 2].copy_from_slice(&(0xF000 | items_len).to_be_bytes());
        }

        psi_list
    }
}

impl From<&Psi> for Nit {
    fn from(psi: &Psi) -> Self {
        let mut nit = Nit::default();
        nit.parse(psi);
        nit
    }
}
