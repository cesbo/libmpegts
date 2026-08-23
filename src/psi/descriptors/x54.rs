use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One entry of a [`Desc54Ref`]: a two-level content classification
/// nibble plus a two-part user byte.
#[derive(Debug, Clone, Copy)]
pub struct Desc54ItemRef {
    nibble: u8,
    user: u8,
}

impl Desc54ItemRef {
    /// Content nibble level 1 (broad classification).
    pub fn content_nibble_level_1(&self) -> u8 {
        (self.nibble & 0xf0) >> 4
    }

    /// Content nibble level 2 (detailed classification).
    pub fn content_nibble_level_2(&self) -> u8 {
        self.nibble & 0x0f
    }

    /// First user-defined nibble.
    pub fn user_nibble_1(&self) -> u8 {
        (self.user & 0xf0) >> 4
    }

    /// Second user-defined nibble.
    pub fn user_nibble_2(&self) -> u8 {
        self.user & 0x0f
    }
}

/// Iterator over the entries of a content_descriptor.
pub struct Desc54ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc54ItemIter<'a> {
    type Item = Desc54ItemRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.data.len() {
            return None;
        }
        let out = Desc54ItemRef {
            nibble: self.data[self.offset],
            user: self.data[self.offset + 1],
        };
        self.offset += 2;
        Some(out)
    }
}

/// content_descriptor (tag `0x54`): a list of content classification entries,
/// each two bytes (a content nibble byte and a user byte).
#[derive(Debug, Clone, Copy)]
pub struct Desc54Ref<'a>(&'a [u8]);

impl<'a> Desc54Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x54;

    /// Iterator over the content classification entries. A trailing odd byte,
    /// if any, is ignored.
    pub fn items(&self) -> Desc54ItemIter<'a> {
        Desc54ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc54Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        Ok(Desc54Ref(descriptor.data()))
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
    fn parses_entries() {
        // 0x10 -> level1=1, level2=0; 0x25 -> level1=2, level2=5.
        let bytes = descriptor(0x54, &[0x10, 0x00, 0x25, 0xff]);
        let cd = Desc54Ref::try_from(first(&bytes)).unwrap();
        let entries: Vec<_> = cd.items().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content_nibble_level_1(), 1);
        assert_eq!(entries[1].content_nibble_level_1(), 2);
        assert_eq!(entries[1].content_nibble_level_2(), 5);
        assert_eq!(entries[1].user_nibble_1(), 0x0f);
    }

    #[test]
    fn ignores_trailing_odd_byte() {
        let bytes = descriptor(0x54, &[0x10, 0x00, 0x20]);
        let cd = Desc54Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(cd.items().count(), 1);
    }
}
