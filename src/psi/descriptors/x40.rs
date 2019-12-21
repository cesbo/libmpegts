// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrapError;

use crate::textcode::StringDVB;


/// The network name descriptor provides the network name in text form.
///
/// EN 300 468 - 6.2.27
#[derive(Debug, Default, Clone)]
pub struct Desc40 {
    /// Network name.
    pub name: StringDVB
}


impl std::convert::TryFrom<&[u8]> for Desc40 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            name: StringDVB::from(&value[2 ..]),
        })
    }
}


impl Desc40 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + self.name.size() }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x40);
        self.name.assemble_sized(buffer);
    }
}


#[cfg(test)]
mod tests {
    use bitwrap::BitWrap;

    use crate::{
        textcode,
        psi::{
            Descriptor,
            Descriptors,
            Desc40,
        },
    };

    static DATA_40: &[u8] = &[0x40, 0x06, 0x01, 0x43, 0x65, 0x73, 0x62, 0x6f];

    #[test]
    fn test_40_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.unpack(DATA_40).unwrap();

        let mut iter = descriptors.iter();
        if let Some(Descriptor::Desc40(desc)) = iter.next() {
            assert_eq!(desc.name, textcode::StringDVB::from_str("Cesbo", 5));
        }
    }

    #[test]
    fn test_40_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc40 {
            name: textcode::StringDVB::from_str("Cesbo", 5)
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_40);
    }
}
