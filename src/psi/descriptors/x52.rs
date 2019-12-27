// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use bitwrap::BitWrap;


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


#[cfg(test)]
mod tests {
    use {
        bitwrap::BitWrap,
        crate::psi::Desc52,
    };

    static DATA: &[u8] = &[0x52, 0x01, 0x02];

    #[test]
    fn test_52_parse() {
        let mut desc = Desc52::default();
        desc.unpack(DATA).unwrap();

        assert_eq!(desc.tag, 2);
    }

    #[test]
    fn test_52_assemble() {
        let desc = Desc52 {
            tag: 2
        };

        let mut buffer: [u8; 256] = [0; 256];
        let result = desc.pack(&mut buffer).unwrap();
        assert_eq!(result, DATA.len());
        assert_eq!(&buffer[.. result], DATA);
    }
}
