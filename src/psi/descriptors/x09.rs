// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::{
    BitWrap,
    BitWrapError,
};


/// The conditional access descriptor is used to specify both system-wide
/// conditional access management information such as EMMs and
/// elementary stream-specific information such as ECMs.
///
/// ISO 13818-1 - 2.6.16
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc09 {
    /// Type of CA system.
    #[bits_skip(8, 0x09)]
    #[bits_skip(8, 0)]

    #[bits(16)]
    pub caid: u16,
    /// PID of the Transport Stream packets which shall contain
    /// either ECM or EMM information for the CA systems.
    #[bits_skip(3, 0b111)]
    #[bits(13)]
    pub pid: u16,
    /// Private data bytes.

    pub data: Vec<u8>
}


impl std::convert::TryFrom<&[u8]> for Desc09 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Desc09::default();
        result.unpack(value)?;
        if value.len() > 6 {
            result.data.extend_from_slice(&value[6 ..]);
        }
        Ok(result)
    }
}


impl Desc09 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + 4 + self.data.len() }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 6, 0x00);
        self.pack(&mut buffer[skip ..]).unwrap();
        buffer[skip + 1] = (self.size() - 2) as u8;
        buffer.extend_from_slice(&self.data.as_slice());
    }
}


#[cfg(test)]
mod tests {
    use bitwrap::BitWrap;

    use crate::psi::{
        Descriptor,
        Descriptors,
        Desc09,
    };

    static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];

    #[test]
    fn test_09_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.unpack(DATA_09).unwrap();

        let mut iter = descriptors.iter();

        if let Some(Descriptor::Desc09(desc)) = iter.next() {
            assert_eq!(desc.caid, 2403);
            assert_eq!(desc.pid, 1281);
            assert_eq!(desc.data, []);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_09_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc09 {
            caid: 2403,
            pid: 1281,
            data: Vec::new()
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_09);
    }
}
