/// The stream identifier descriptor may be used in the PSI PMT to label
/// component streams of a service so that they can be differentiated,
/// e.g. by text descriptions given in component descriptors in the EIT if present.
///
/// EN 300 468 - 6.2.39
#[derive(Debug, Default)]
pub struct Desc52 {
    /// Identifies the component stream for associating it
    /// with a description given in a component descriptor.
    pub tag: u8
}

impl Desc52 {
    #[inline]
    pub fn min_size() -> usize {
        3
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() == Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        Self {
            tag: slice[2]
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::min_size()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x52);
        buffer.push((self.size() - 2) as u8);
        buffer.push(self.tag);
    }
}
