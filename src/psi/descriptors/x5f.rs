use crate::psi::{
    Descriptor,
    DescriptorRef,
    PsiSectionError,
};

/// private_data_specifier_descriptor (tag `0x5f`): identifies the specifier
/// of the private descriptors that follow in the same descriptor loop.
#[derive(Debug, Clone, Copy)]
pub struct PrivateDataSpecifierDescriptorRef<'a>(&'a [u8]);

impl<'a> PrivateDataSpecifierDescriptorRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x5f;

    /// Private data specifier registered by DVB.
    pub fn specifier(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for PrivateDataSpecifierDescriptorRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 4 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(PrivateDataSpecifierDescriptorRef(data))
    }
}

/// private_data_specifier_descriptor (tag `0x5f`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct PrivateDataSpecifierDescriptor {
    /// Private data specifier registered by DVB
    pub specifier: u32,
}

impl Descriptor for PrivateDataSpecifierDescriptor {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        dst.push(PrivateDataSpecifierDescriptorRef::TAG);
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
        PrivateDataSpecifierDescriptor { specifier: 0x29 }
            .encode(&mut dst)
            .unwrap();

        assert_eq!(dst, [0x5f, 0x04, 0x00, 0x00, 0x00, 0x29]);

        let pds = PrivateDataSpecifierDescriptorRef::try_from(first(&dst)).unwrap();
        assert_eq!(pds.specifier(), 0x29);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x5e, 0x04, 0x00, 0x00, 0x00, 0x28];
        assert!(PrivateDataSpecifierDescriptorRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x5f, 0x03, 0x00, 0x00, 0x28];
        assert!(PrivateDataSpecifierDescriptorRef::try_from(first(&bytes)).is_err());
    }
}
