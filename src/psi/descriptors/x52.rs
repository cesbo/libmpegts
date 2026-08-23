use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// stream_identifier_descriptor (tag `0x52`): tags one service component for
/// cross-referencing from SI tables.
#[derive(Debug, Clone, Copy)]
pub struct Desc52Ref<'a>(&'a [u8]);

impl<'a> Desc52Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x52;

    /// Component tag, unique within the program.
    pub fn component_tag(&self) -> u8 {
        self.0[0]
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc52Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 1 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc52Ref(data))
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
    fn parses_component_tag() {
        let bytes = [0x52, 0x01, 0x21];

        let stream = Desc52Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(stream.component_tag(), 0x21);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x53, 0x01, 0x21];
        assert!(Desc52Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x52, 0x02, 0x21, 0x22];
        assert!(Desc52Ref::try_from(first(&bytes)).is_err());
    }
}
