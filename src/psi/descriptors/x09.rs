// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


/// The conditional access descriptor is used to specify both system-wide
/// conditional access management information such as EMMs and
/// elementary stream-specific information such as ECMs.
///
/// ISO 13818-1 - 2.6.16
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc09 {
    #[bits(8, skip = 0x09)]
    #[bits(8, name = desc_len, value = self.size() - 2)]

    #[bits(16)]
    pub caid: u16,
    /// PID of the Transport Stream packets which shall contain
    /// either ECM or EMM information for the CA systems.
    #[bits(3, skip = 0b111)]
    #[bits(13)]
    pub pid: u16,

    /// Private data bytes.
    #[bytes(desc_len - 4)]
    pub data: Vec<u8>
}


impl Desc09 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + 4 + self.data.len() }
}


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::psi::Desc09,
    };

    static DATA: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];

    #[test]
    fn test_09_unpack() {
        let mut desc = Desc09::default();
        desc.unpack(DATA).unwrap();

        assert_eq!(desc.caid, 2403);
        assert_eq!(desc.pid, 1281);
        assert_eq!(desc.data, []);
    }

    #[test]
    fn test_09_pack() {
        let desc = Desc09 {
            caid: 2403,
            pid: 1281,
            data: Vec::new(),
        };

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
