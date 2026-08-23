use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One entry of a [`Desc59Ref`]: a language code plus DVB subtitle stream
/// reference.
#[derive(Debug, Clone, Copy)]
pub struct Desc59ItemRef<'a> {
    data: &'a [u8],
}

impl<'a> Desc59ItemRef<'a> {
    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        &self.data[.. 3]
    }

    /// Subtitling type (component_type of stream_content `0x03`), e.g.
    /// `0x10 ..= 0x15` normal, `0x20 ..= 0x25` hearing-impaired.
    pub fn subtitling_type(&self) -> u8 {
        self.data[3]
    }

    /// Composition page ID.
    pub fn composition_page_id(&self) -> u16 {
        u16::from_be_bytes([self.data[4], self.data[5]])
    }

    /// Ancillary page ID.
    pub fn ancillary_page_id(&self) -> u16 {
        u16::from_be_bytes([self.data[6], self.data[7]])
    }
}

/// Iterator over the entries of a subtitling_descriptor.
pub struct Desc59ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc59ItemIter<'a> {
    type Item = Desc59ItemRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 8 > self.data.len() {
            return None;
        }
        let out = Desc59ItemRef {
            data: &self.data[self.offset .. self.offset + 8],
        };
        self.offset += 8;
        Some(out)
    }
}

/// subtitling_descriptor (tag `0x59`): DVB subtitle streams carried in the
/// elementary stream.
#[derive(Debug, Clone, Copy)]
pub struct Desc59Ref<'a>(&'a [u8]);

impl<'a> Desc59Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x59;

    /// Iterator over subtitle stream entries.
    pub fn items(&self) -> Desc59ItemIter<'a> {
        Desc59ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc59Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(8) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc59Ref(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::DescriptorsRef;

    fn first(bytes: &[u8]) -> DescriptorRef<'_> {
        DescriptorsRef::from(bytes)
            .into_iter()
            .next()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_subtitling_items() {
        let bytes = [
            0x59, 0x08, 0x66, 0x72, 0x61, 0x10, 0x00, 0x01, 0x00, 0x02,
        ];

        let subtitling = Desc59Ref::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = subtitling.items().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].lang(), b"fra");
        assert_eq!(items[0].subtitling_type(), 0x10);
        assert_eq!(items[0].composition_page_id(), 1);
        assert_eq!(items[0].ancillary_page_id(), 2);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x56, 0x08, 0x66, 0x72, 0x61, 0x10, 0x00, 0x01, 0x00, 0x02];
        assert!(Desc59Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x59, 0x04, 0x66, 0x72, 0x61, 0x10];
        assert!(Desc59Ref::try_from(first(&bytes)).is_err());
    }
}
