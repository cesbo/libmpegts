use crate::{
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    utils::textcode::{
        self,
        Charset,
        DvbTextRef,
    },
};

/// network_name_descriptor (tag `0x40`): DVB-coded network name.
#[derive(Debug, Clone, Copy)]
pub struct Desc40Ref<'a>(&'a [u8]);

impl<'a> Desc40Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x40;

    /// Raw, DVB-coded network name bytes.
    pub fn name(&self) -> &'a [u8] {
        self.0
    }

    /// Network name decoded according to DVB text coding.
    pub fn name_text(&self) -> Result<DvbTextRef<'a>, textcode::Error> {
        DvbTextRef::try_from(self.name())
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc40Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        Ok(Desc40Ref(descriptor.data()))
    }
}

/// network_name_descriptor (tag `0x40`) encoder. The name is DVB-coded with
/// `charset`.
#[derive(Debug, Clone, Copy)]
pub struct Desc40<'a> {
    pub name: &'a str,
    pub charset: Charset,
}

impl Descriptor for Desc40<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        let name = textcode::encode(self.name, self.charset);
        if name.len() > 0xff {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        dst.push(Desc40Ref::TAG);
        dst.push(name.len() as u8);
        dst.extend_from_slice(&name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::DescriptorsRef;

    fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![tag, payload.len() as u8];
        v.extend_from_slice(payload);
        v
    }

    fn first(bytes: &[u8]) -> DescriptorRef<'_> {
        DescriptorsRef::from(bytes)
            .into_iter()
            .next()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_network_name() {
        let bytes = descriptor(0x40, b"Astra");

        let network = Desc40Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(network.name(), b"Astra");
        assert_eq!(network.name_text().unwrap().to_string(), "Astra");
    }

    #[test]
    fn accepts_empty_name() {
        let bytes = descriptor(0x40, b"");

        let network = Desc40Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(network.name(), b"");
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x41, b"Astra");
        assert!(Desc40Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn encodes_network_name() {
        let mut dst = Vec::new();
        Desc40 {
            name: "Astra",
            charset: Charset::Iso6937,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, b"\x40\x05Astra");
    }

    #[test]
    fn encodes_name_with_charset_header() {
        let mut dst = Vec::new();
        Desc40 {
            name: "Сеть",
            charset: Charset::Iso8859_5,
        }
        .encode(&mut dst)
        .unwrap();

        let network = Desc40Ref::try_from(first(&dst)).unwrap();
        assert_eq!(network.name()[0], 0x01);
        assert_eq!(network.name_text().unwrap().to_string(), "Сеть");
    }

    #[test]
    fn encode_rejects_oversized_name() {
        let name = "n".repeat(256);
        let mut dst = Vec::new();
        let result = Desc40 {
            name: &name,
            charset: Charset::Iso6937,
        }
        .encode(&mut dst);

        assert!(result.is_err());
        assert!(dst.is_empty());
    }
}
