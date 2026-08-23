use crate::psi::{
    Descriptor,
    DescriptorRef,
    PsiSectionError,
};

/// One entry of a service_list_descriptor: service_id plus service type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceListItem {
    pub service_id: u16,
    pub service_type: u8,
}

/// Iterator over the entries of a service_list_descriptor.
pub struct ServiceListItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl Iterator for ServiceListItemIter<'_> {
    type Item = ServiceListItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 3 > self.data.len() {
            return None;
        }
        let out = ServiceListItem {
            service_id: u16::from_be_bytes([self.data[self.offset], self.data[self.offset + 1]]),
            service_type: self.data[self.offset + 2],
        };
        self.offset += 3;
        Some(out)
    }
}

/// service_list_descriptor (tag `0x41`): services carried in the transport
/// stream, each as service_id plus service type.
#[derive(Debug, Clone, Copy)]
pub struct ServiceListDescriptorRef<'a>(&'a [u8]);

impl<'a> ServiceListDescriptorRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x41;

    /// Iterator over service entries.
    pub fn items(&self) -> ServiceListItemIter<'a> {
        ServiceListItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for ServiceListDescriptorRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(3) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(ServiceListDescriptorRef(data))
    }
}

// 85 3-byte entries fit the 8-bit descriptor length
const SERVICE_LIST_CHUNK: usize = 0xff / 3;

/// service_list_descriptor (tag `0x41`) encoder. More than 85 entries are
/// split into repeated descriptors; an empty list appends nothing.
#[derive(Debug, Clone, Copy)]
pub struct ServiceListDescriptor<'a> {
    pub items: &'a [ServiceListItem],
}

impl Descriptor for ServiceListDescriptor<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        for chunk in self.items.chunks(SERVICE_LIST_CHUNK) {
            dst.push(ServiceListDescriptorRef::TAG);
            dst.push((chunk.len() * 3) as u8);
            for item in chunk {
                dst.extend_from_slice(&item.service_id.to_be_bytes());
                dst.push(item.service_type);
            }
        }
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
    fn parses_service_items() {
        let bytes = descriptor(0x41, &[0x00, 0x01, 0x01, 0x27, 0x11, 0x19]);

        let list = ServiceListDescriptorRef::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = list.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].service_id, 1);
        assert_eq!(items[0].service_type, 1);
        assert_eq!(items[1].service_id, 0x2711);
        assert_eq!(items[1].service_type, 0x19);
    }

    #[test]
    fn accepts_empty_payload() {
        let bytes = descriptor(0x41, &[]);

        let list = ServiceListDescriptorRef::try_from(first(&bytes)).unwrap();
        assert_eq!(list.items().count(), 0);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x40, &[0x00, 0x01, 0x01]);
        assert!(ServiceListDescriptorRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = descriptor(0x41, &[0x00, 0x01, 0x01, 0x27]);
        assert!(ServiceListDescriptorRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn encodes_service_items() {
        let mut dst = Vec::new();
        ServiceListDescriptor {
            items: &[
                ServiceListItem {
                    service_id: 1,
                    service_type: 1,
                },
                ServiceListItem {
                    service_id: 0x2711,
                    service_type: 0x19,
                },
            ],
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, [0x41, 0x06, 0x00, 0x01, 0x01, 0x27, 0x11, 0x19]);
    }

    #[test]
    fn encodes_empty_list_as_nothing() {
        let mut dst = Vec::new();
        ServiceListDescriptor { items: &[] }
            .encode(&mut dst)
            .unwrap();
        assert!(dst.is_empty());
    }

    #[test]
    fn splits_oversized_list_into_repeated_descriptors() {
        let items: Vec<ServiceListItem> = (1 ..= 86)
            .map(|i| ServiceListItem {
                service_id: i,
                service_type: 1,
            })
            .collect();

        let mut dst = Vec::new();
        ServiceListDescriptor { items: &items }
            .encode(&mut dst)
            .unwrap();

        let mut collected = Vec::new();
        for descriptor in DescriptorsRef::from(&dst[..]) {
            let list = ServiceListDescriptorRef::try_from(descriptor.unwrap()).unwrap();
            collected.push(list.items().count());
        }
        assert_eq!(collected, [85, 1]);
    }
}
