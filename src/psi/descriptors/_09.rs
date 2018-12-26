use base;


/// The conditional access descriptor is used to specify both system-wide
/// conditional access management information such as EMMs and
/// elementary stream-specific information such as ECMs.
///
/// ISO 13818-1 - 2.6.16
#[derive(Debug, Default)]
pub struct Desc09 {
    /// Type of CA system.
    pub caid: u16,
    /// PID of the Transport Stream packets which shall contain
    /// either ECM or EMM information for the CA systems.
    pub pid: u16,
    /// Private data bytes.
    pub data: Vec<u8>
}

impl Desc09 {
    #[inline]
    pub fn min_size() -> usize {
        6
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            caid: base::get_u16(&slice[2 ..]),
            pid: base::get_pid(&slice[4 ..]),
            data: Vec::from(&slice[6 ..]),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size() + self.data.len()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x09);
        buffer.push((self.size() - 2) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u16(&mut buffer[skip ..], self.caid);
        base::set_pid(&mut buffer[skip + 2 ..], self.pid);
        buffer.extend_from_slice(&self.data.as_slice());
    }
}
