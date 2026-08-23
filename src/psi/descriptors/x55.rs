use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One entry of a [`Desc55Ref`]: a country code plus rating value.
#[derive(Debug, Clone, Copy)]
pub struct Desc55ItemRef<'a> {
    country_code: &'a [u8],
    rating: u8,
}

impl<'a> Desc55ItemRef<'a> {
    /// 3-byte ISO 3166 country code.
    pub fn country_code(&self) -> &'a [u8] {
        self.country_code
    }

    /// Rating value:
    /// * `0x00` - undefined
    /// * `0x01 ..= 0x0f` - minimum age = rating + 3 years
    /// * `0x10 ..` - broadcaster-defined
    pub fn rating(&self) -> u8 {
        self.rating
    }
}

/// Iterator over the entries of a parental_rating_descriptor.
pub struct Desc55ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc55ItemIter<'a> {
    type Item = Desc55ItemRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 4 > self.data.len() {
            return None;
        }
        let out = Desc55ItemRef {
            country_code: &self.data[self.offset .. self.offset + 3],
            rating: self.data[self.offset + 3],
        };
        self.offset += 4;
        Some(out)
    }
}

/// parental_rating_descriptor (tag `0x55`): age ratings of an event per
/// country.
#[derive(Debug, Clone, Copy)]
pub struct Desc55Ref<'a>(&'a [u8]);

impl<'a> Desc55Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x55;

    /// Iterator over country/rating entries.
    pub fn items(&self) -> Desc55ItemIter<'a> {
        Desc55ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc55Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(4) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc55Ref(data))
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
    fn parses_rating_items() {
        let bytes = [0x55, 0x08, 0x42, 0x55, 0x4c, 0x0f, 0x47, 0x42, 0x52, 0x00];

        let rating = Desc55Ref::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = rating.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].country_code(), b"BUL");
        assert_eq!(items[0].rating(), 0x0f);
        assert_eq!(items[1].country_code(), b"GBR");
        assert_eq!(items[1].rating(), 0x00);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x54, 0x04, 0x42, 0x55, 0x4c, 0x0f];
        assert!(Desc55Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x55, 0x03, 0x42, 0x55, 0x4c];
        assert!(Desc55Ref::try_from(first(&bytes)).is_err());
    }
}
