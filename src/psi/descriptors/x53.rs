use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// Iterator over the CA system IDs of a CA_identifier_descriptor.
pub struct Desc53ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl Iterator for Desc53ItemIter<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.data.len() {
            return None;
        }
        let out = u16::from_be_bytes([self.data[self.offset], self.data[self.offset + 1]]);
        self.offset += 2;
        Some(out)
    }
}

/// CA_identifier_descriptor (tag `0x53`): CA systems associated with a
/// service or event, without binding to EMM/ECM PIDs.
#[derive(Debug, Clone, Copy)]
pub struct Desc53Ref<'a>(&'a [u8]);

impl<'a> Desc53Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x53;

    /// Iterator over CA system IDs.
    pub fn items(&self) -> Desc53ItemIter<'a> {
        Desc53ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc53Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(2) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc53Ref(data))
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
    fn parses_ca_system_ids() {
        let bytes = [0x53, 0x04, 0x09, 0x63, 0x06, 0x02];

        let ca = Desc53Ref::try_from(first(&bytes)).unwrap();
        let items: Vec<u16> = ca.items().collect();
        assert_eq!(items, [0x0963, 0x0602]);
    }

    #[test]
    fn accepts_empty_payload() {
        let bytes = [0x53, 0x00];

        let ca = Desc53Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(ca.items().count(), 0);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x52, 0x02, 0x09, 0x63];
        assert!(Desc53Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x53, 0x03, 0x09, 0x63, 0x06];
        assert!(Desc53Ref::try_from(first(&bytes)).is_err());
    }
}
