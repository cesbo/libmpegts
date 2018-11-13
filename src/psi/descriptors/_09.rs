use base;


/// The conditional access descriptor is used to specify both system-wide
/// conditional access management information such as EMMs and
/// elementary stream-specific information such as ECMs.
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
            pid: base::get_u13(&slice[4 ..]),
            data: Vec::from(&slice[6 ..]),
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x09);
        buffer.push((Self::min_size() - 2 + 4 + self.data.len()) as u8);

        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        {
            let ptr = buffer.as_mut_slice();
            base::set_u16(&mut ptr[skip ..], self.caid);
            base::set_u16(&mut ptr[skip + 2 ..], 0xE000 | self.pid);
        }
        buffer.extend_from_slice(&self.data.as_slice());
    }
}
