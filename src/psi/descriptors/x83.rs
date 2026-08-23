use crate::{
    pack_bits,
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
};

/// Bit layout of logical_channel_descriptor entries, selected by the
/// private_data_specifier_descriptor preceding it in the descriptor loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcnFormat {
    /// visible(1) + reserved(5) + 10-bit LCN; private_data_specifier `0x28`
    Eacem,
    /// visible(1) + reserved(1) + 14-bit LCN; private_data_specifier `0x29`
    NordigV1,
}

/// One entry of a logical_channel_descriptor: service_id, visibility and
/// logical channel number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Desc83Item {
    pub service_id: u16,
    pub visible: bool,
    pub lcn: u16,
}

/// Iterator over the entries of a logical_channel_descriptor.
pub struct Desc83ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
    format: LcnFormat,
}

impl Iterator for Desc83ItemIter<'_> {
    type Item = Desc83Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 4 > self.data.len() {
            return None;
        }
        let lcn = u16::from_be_bytes([self.data[self.offset + 2], self.data[self.offset + 3]]);
        let out = Desc83Item {
            service_id: u16::from_be_bytes([self.data[self.offset], self.data[self.offset + 1]]),
            visible: (self.data[self.offset + 2] & 0x80) != 0,
            lcn: match self.format {
                LcnFormat::Eacem => lcn & 0x03ff,
                LcnFormat::NordigV1 => lcn & 0x3fff,
            },
        };
        self.offset += 4;
        Some(out)
    }
}

/// logical_channel_descriptor (tag `0x83`, private): channel numbers assigned
/// to services.
#[derive(Debug, Clone, Copy)]
pub struct Desc83Ref<'a>(&'a [u8]);

impl<'a> Desc83Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x83;

    /// Iterator over channel entries, decoded according to `format`.
    pub fn items(&self, format: LcnFormat) -> Desc83ItemIter<'a> {
        Desc83ItemIter {
            data: self.0,
            offset: 0,
            format,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc83Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(4) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc83Ref(data))
    }
}

// 63 4-byte entries fit the 8-bit descriptor length
const LOGICAL_CHANNEL_CHUNK: usize = 0xff / 4;

/// logical_channel_descriptor (tag `0x83`) encoder. More than 63 entries are
/// split into repeated descriptors; an empty list appends nothing.
#[derive(Debug, Clone, Copy)]
pub struct Desc83<'a> {
    pub format: LcnFormat,
    pub items: &'a [Desc83Item],
}

impl Descriptor for Desc83<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        for chunk in self.items.chunks(LOGICAL_CHANNEL_CHUNK) {
            dst.push(Desc83Ref::TAG);
            dst.push((chunk.len() * 4) as u8);
            for item in chunk {
                dst.extend_from_slice(&item.service_id.to_be_bytes());
                dst.extend_from_slice(&match self.format {
                    LcnFormat::Eacem => pack_bits!(u16,
                        visible: 1 => item.visible,
                        reserved: 5 => 0b11111,
                        lcn: 10 => item.lcn,
                    ),
                    LcnFormat::NordigV1 => pack_bits!(u16,
                        visible: 1 => item.visible,
                        reserved: 1 => 1,
                        lcn: 14 => item.lcn,
                    ),
                });
            }
        }
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
    fn encodes_eacem_items() {
        let mut dst = Vec::new();
        Desc83 {
            format: LcnFormat::Eacem,
            items: &[
                Desc83Item {
                    service_id: 1,
                    visible: true,
                    lcn: 5,
                },
                Desc83Item {
                    service_id: 2,
                    visible: false,
                    lcn: 1000,
                },
            ],
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(
            dst,
            [0x83, 0x08, 0x00, 0x01, 0xfc, 0x05, 0x00, 0x02, 0x7f, 0xe8]
        );
    }

    #[test]
    fn encodes_nordig_items() {
        let mut dst = Vec::new();
        Desc83 {
            format: LcnFormat::NordigV1,
            items: &[Desc83Item {
                service_id: 1,
                visible: true,
                lcn: 1500,
            }],
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, [0x83, 0x04, 0x00, 0x01, 0xc5, 0xdc]);
    }

    #[test]
    fn roundtrips_both_formats() {
        let items = [
            Desc83Item {
                service_id: 0x1234,
                visible: true,
                lcn: 999,
            },
            Desc83Item {
                service_id: 2,
                visible: false,
                lcn: 1,
            },
        ];

        for format in [LcnFormat::Eacem, LcnFormat::NordigV1] {
            let mut dst = Vec::new();
            Desc83 {
                format,
                items: &items,
            }
            .encode(&mut dst)
            .unwrap();

            let desc = Desc83Ref::try_from(first(&dst)).unwrap();
            let decoded: Vec<_> = desc.items(format).collect();
            assert_eq!(decoded, items, "{:?}", format);
        }
    }

    #[test]
    fn nordig_keeps_wide_lcn() {
        // 14-bit LCN above the 10-bit EACEM range
        let items = [Desc83Item {
            service_id: 1,
            visible: true,
            lcn: 8500,
        }];

        let mut dst = Vec::new();
        Desc83 {
            format: LcnFormat::NordigV1,
            items: &items,
        }
        .encode(&mut dst)
        .unwrap();

        let desc = Desc83Ref::try_from(first(&dst)).unwrap();
        assert_eq!(desc.items(LcnFormat::NordigV1).next().unwrap().lcn, 8500);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x84, 0x04, 0x00, 0x01, 0xfc, 0x05];
        assert!(Desc83Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x83, 0x03, 0x00, 0x01, 0xfc];
        assert!(Desc83Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn splits_oversized_list_into_repeated_descriptors() {
        let items: Vec<Desc83Item> = (1 ..= 64)
            .map(|i| Desc83Item {
                service_id: i,
                visible: true,
                lcn: i,
            })
            .collect();

        let mut dst = Vec::new();
        Desc83 {
            format: LcnFormat::Eacem,
            items: &items,
        }
        .encode(&mut dst)
        .unwrap();

        let mut collected = Vec::new();
        for descriptor in DescriptorsRef::from(&dst[..]) {
            let desc = Desc83Ref::try_from(descriptor.unwrap()).unwrap();
            collected.push(desc.items(LcnFormat::Eacem).count());
        }
        assert_eq!(collected, [63, 1]);
    }
}
