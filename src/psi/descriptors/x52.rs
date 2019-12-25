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


/// The stream identifier descriptor may be used in the PSI PMT to label
/// component streams of a service so that they can be differentiated,
/// e.g. by text descriptions given in component descriptors in the EIT if present.
///
/// EN 300 468 - 6.2.39
#[derive(Debug, Default, Clone, BitWrap)]
pub struct Desc52 {
    #[bits(8, skip = 0x52)]
    #[bits(8, skip = 1)]

    /// Identifies the component stream for associating it
    /// with a description given in a component descriptor.
    #[bits(8)]
    pub tag: u8
}


impl std::convert::TryFrom<&[u8]> for Desc52 {
    type Error = BitWrapError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        result.unpack(value)?;
        Ok(result)
    }
}


impl Desc52 {
    #[inline]
    pub (crate) fn size(&self) -> usize { 2 + 1 }

    pub (crate) fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 2 + 1, 0x00);
        self.pack(&mut buffer[skip ..]).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use {
        std::convert::TryFrom,
        crate::psi::Desc52,
    };

    static DATA: &[u8] = &[0x52, 0x01, 0x02];

    #[test]
    fn test_52_parse() {
        let desc = Desc52::try_from(DATA).unwrap();

        assert_eq!(desc.tag, 2);
    }

    #[test]
    fn test_52_assemble() {
        let desc = Desc52 {
            tag: 2
        };

        let mut assembled = Vec::new();
        desc.assemble(&mut assembled);
        assert_eq!(assembled.as_slice(), DATA);
    }
}
