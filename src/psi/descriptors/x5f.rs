use crate::psi::{
    Descriptor,
    DescriptorRef,
    PsiSectionError,
};

/// private_data_specifier_descriptor (tag `0x5f`): identifies the specifier
/// of the private descriptors that follow in the same descriptor loop.
#[derive(Debug, Clone, Copy)]
pub struct Desc5FRef<'a>(&'a [u8]);

impl<'a> Desc5FRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x5f;

    /// Private data specifier registered by DVB.
    pub fn specifier(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc5FRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 4 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc5FRef(data))
    }
}

/// private_data_specifier_descriptor (tag `0x5f`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct Desc5F {
    /// Private data specifier registered by DVB
    pub specifier: u32,
}

impl Descriptor for Desc5F {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        dst.push(Desc5FRef::TAG);
        dst.push(4);
        dst.extend_from_slice(&self.specifier.to_be_bytes());
        Ok(())
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
    fn encodes_private_data_specifier() {
        let mut dst = Vec::new();
        Desc5F { specifier: 0x29 }
            .encode(&mut dst)
            .unwrap();

        assert_eq!(dst, [0x5f, 0x04, 0x00, 0x00, 0x00, 0x29]);

        let pds = Desc5FRef::try_from(first(&dst)).unwrap();
        assert_eq!(pds.specifier(), 0x29);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x5e, 0x04, 0x00, 0x00, 0x00, 0x28];
        assert!(Desc5FRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x5f, 0x03, 0x00, 0x00, 0x28];
        assert!(Desc5FRef::try_from(first(&bytes)).is_err());
    }
}
