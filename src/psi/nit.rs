use crate::bytes::*;
use crate::psi::{Psi, PsiDemux, Descriptors};

pub const NIT_PID: u16 = 0x0010;

/// Maximum section length, exclude PSI header and CRC
const NIT_MAX_SIZE: usize = 1024 - 3 - 4;

/// NIT Item.
#[derive(Debug, Default)]
pub struct NitItem {
    /// Identifier which serves as a label for identification of this
    /// TS from any other multiplex within the delivery system.
    pub tsid: u16,
    /// Label identifying the network_id of the originating delivery system.
    pub onid: u16,
    /// List of descriptors.
    pub descriptors: Descriptors
}

impl NitItem {
    pub fn parse(slice: &[u8]) -> Self {
        let mut item = Self::default();

        item.tsid = slice[0 ..].get_u16();
        item.onid = slice[2 ..].get_u16();

        item.descriptors.parse(&slice[6 ..]);

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 6, 0x00);

        buffer[skip ..].set_u16(self.tsid);
        buffer[skip + 2 ..].set_u16(self.onid);

        let descriptors_len = self.descriptors.assemble(buffer) as u16;
        buffer[skip + 4 ..].set_u16(0xF000 | descriptors_len);
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
    pub items: Vec<NitItem>
}

impl Nit {
    #[inline]
    pub fn check(&self, psi: &Psi) -> bool {
        psi.size >= 12 + 4 &&
        (psi.buffer[0] & 0xFE) == 0x40 && /* 0x40 or 0x41 */
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(psi) {
            return;
        }

        self.table_id = psi.buffer[0];
        self.network_id = psi.buffer[3 ..].get_u16();
        self.version = (psi.buffer[5] & 0x3E) >> 1;

        let descriptors_len = (psi.buffer[8 ..].get_u16() & 0x0FFF) as usize;
        self.descriptors.parse(&psi.buffer[10 .. 10 + descriptors_len]);

        let ptr = &psi.buffer[12 + descriptors_len .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 6 {
            let item_len = 6 + (ptr[skip + 4 ..].get_u16() & 0x0FFF) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(NitItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    fn psi_init(&self, first: bool) -> Psi {
        let mut psi = Psi::new(self.table_id, 10, self.version);
        psi.buffer[1] = 0xF0;  // set section_syntax_indicator and reserved bits
        psi.buffer[3 ..].set_u16(self.network_id);
        if first {
            let descriptors_len = self.descriptors.assemble(&mut psi.buffer) as u16;
            psi.buffer[8 ..].set_u16(0xF000 | descriptors_len);
        } else {
            psi.buffer[8] = 0xF0;  //reserved
        }
        // transport_stream_loop_lengt
        psi.buffer.push(0x00);
        psi.buffer.push(0x00);
        psi
    }
}

impl PsiDemux for Nit {
    fn psi_list_assemble(&self) -> Vec<Psi> {
        let mut psi_list = vec![self.psi_init(true)];

        for item in &self.items {
            {
                let psi = psi_list.last_mut().unwrap();
                if NIT_MAX_SIZE >= psi.buffer.len() + item.size() {
                    item.assemble(&mut psi.buffer);
                    continue;
                }
            }

            let mut psi = self.psi_init(false);
            item.assemble(&mut psi.buffer);
            psi_list.push(psi);
        }

        for item in &mut psi_list {
            let descriptors_len = (item.buffer[8 ..].get_u16() & 0x0FFF) as usize;
            let items_len = (item.buffer.len() - 12 - descriptors_len) as u16;
            let skip = 10 + descriptors_len;
            item.buffer[skip ..].set_u16(0xF000 | items_len);
        }

        psi_list
    }
}
