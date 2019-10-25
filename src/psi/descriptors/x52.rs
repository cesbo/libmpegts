// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use super::Desc;


const MIN_SIZE: usize = 3;


/// The stream identifier descriptor may be used in the PSI PMT to label
/// component streams of a service so that they can be differentiated,
/// e.g. by text descriptions given in component descriptors in the EIT if present.
///
/// EN 300 468 - 6.2.39
#[derive(Debug, Default, Clone)]
pub struct Desc52 {
    /// Identifies the component stream for associating it
    /// with a description given in a component descriptor.
    pub tag: u8
}


impl Desc52 {
    pub fn check(slice: &[u8]) -> bool {
        slice.len() == MIN_SIZE
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            tag: slice[2]
        }
    }
}


impl Desc for Desc52 {
    #[inline]
    fn tag(&self) -> u8 {
        0x52
    }

    #[inline]
    fn size(&self) -> usize {
        MIN_SIZE
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x52);
        buffer.push((self.size() - 2) as u8);
        buffer.push(self.tag);
    }
}


#[cfg(test)]
mod tests {
    use crate::psi::{
        Descriptors,
        Desc52,
    };

    static DATA_52: &[u8] = &[0x52, 0x01, 0x02];

    #[test]
    fn test_52_parse() {
        let mut descriptors = Descriptors::default();
        descriptors.parse(DATA_52);

        let desc = descriptors.iter().next().unwrap().downcast_ref::<Desc52>();
        assert_eq!(desc.tag, 2);
    }

    #[test]
    fn test_52_assemble() {
        let mut descriptors = Descriptors::default();
        descriptors.push(Desc52 {
            tag: 2
        });

        let mut assembled = Vec::new();
        descriptors.assemble(&mut assembled);

        assert_eq!(assembled.as_slice(), DATA_52);
    }
}
