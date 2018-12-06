use base;
use psi::{Psi, Descriptors};


pub const NIT_PID: u16 = 0x10;


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
        Self::default()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        ()
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
        match psi.buffer[0] {
            0x10 => true,
            _ => false
        } &&
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        ()
    }

    pub fn assemble(&self, psi: &mut Psi) {
        ()
    }
}
