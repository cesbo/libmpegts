use crate::{
    pack_bits,
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    utils::{
        BcdTime,
        MjdFrom,
        MjdTo,
    },
};

const LOCAL_TIME_OFFSET_ITEM_SIZE: usize = 13;

/// One entry of a local_time_offset_descriptor: the current and the next
/// UTC offset of one country or region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Desc58Item {
    /// ISO 3166 alpha-3 country code
    pub country_code: [u8; 3],
    /// Zero when the country has a single time zone
    pub country_region_id: u8,
    /// `true` when local time is behind UTC (the offset is subtracted)
    pub local_time_offset_polarity: bool,
    /// Offset from UTC in minutes
    pub local_time_offset: u16,
    /// Unix timestamp of the next offset change
    pub time_of_change: u64,
    /// Offset from UTC in minutes after the change
    pub next_time_offset: u16,
}

/// Iterator over the entries of a local_time_offset_descriptor.
pub struct Desc58ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl Iterator for Desc58ItemIter<'_> {
    type Item = Desc58Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + LOCAL_TIME_OFFSET_ITEM_SIZE > self.data.len() {
            return None;
        }
        let d = &self.data[self.offset ..];
        let out = Desc58Item {
            country_code: [d[0], d[1], d[2]],
            country_region_id: (d[3] & 0xfc) >> 2,
            local_time_offset_polarity: (d[3] & 0x01) != 0,
            local_time_offset: u16::from_bcd_time([d[4], d[5]]),
            time_of_change: u64::from_mjd([d[6], d[7]]) + u64::from_bcd_time([d[8], d[9], d[10]]),
            next_time_offset: u16::from_bcd_time([d[11], d[12]]),
        };
        self.offset += LOCAL_TIME_OFFSET_ITEM_SIZE;
        Some(out)
    }
}

/// local_time_offset_descriptor (tag `0x58`): UTC offsets of countries and
/// regions and the time of the next offset change.
#[derive(Debug, Clone, Copy)]
pub struct Desc58Ref<'a>(&'a [u8]);

impl<'a> Desc58Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x58;

    /// Iterator over local time offset entries.
    pub fn items(&self) -> Desc58ItemIter<'a> {
        Desc58ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc58Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(LOCAL_TIME_OFFSET_ITEM_SIZE) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc58Ref(data))
    }
}

// 19 13-byte entries fit the 8-bit descriptor length
const LOCAL_TIME_OFFSET_CHUNK: usize = 0xff / LOCAL_TIME_OFFSET_ITEM_SIZE;

/// local_time_offset_descriptor (tag `0x58`) encoder. More than 19 entries
/// are split into repeated descriptors; an empty list appends nothing.
#[derive(Debug, Clone, Copy)]
pub struct Desc58<'a> {
    pub items: &'a [Desc58Item],
}

impl Descriptor for Desc58<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        for chunk in self.items.chunks(LOCAL_TIME_OFFSET_CHUNK) {
            dst.push(Desc58Ref::TAG);
            dst.push((chunk.len() * LOCAL_TIME_OFFSET_ITEM_SIZE) as u8);
            for item in chunk {
                // BCD time fields hold at most 23:59
                debug_assert!(item.local_time_offset < 1440);
                debug_assert!(item.next_time_offset < 1440);

                dst.extend_from_slice(&item.country_code);
                dst.extend_from_slice(&pack_bits!(u8,
                    country_region_id: 6 => item.country_region_id,
                    reserved: 1 => 1,
                    local_time_offset_polarity: 1 => item.local_time_offset_polarity,
                ));
                dst.extend_from_slice(&item.local_time_offset.into_bcd_time());
                dst.extend_from_slice(&item.time_of_change.into_mjd());
                dst.extend_from_slice(&item.time_of_change.into_bcd_time());
                dst.extend_from_slice(&item.next_time_offset.into_bcd_time());
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
    fn encodes_local_time_offset() {
        let mut dst = Vec::new();
        Desc58 {
            items: &[Desc58Item {
                country_code: *b"BUL",
                country_region_id: 0,
                local_time_offset_polarity: false,
                local_time_offset: 120,
                // 2027-01-15 08:00:00 UTC, MJD 61420
                time_of_change: 1_800_000_000,
                next_time_offset: 180,
            }],
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(
            dst,
            [
                0x58, 0x0d, 0x42, 0x55, 0x4c, 0x02, 0x02, 0x00, 0xef, 0xec, 0x08, 0x00, 0x00,
                0x03, 0x00
            ]
        );
    }

    #[test]
    fn roundtrips_local_time_offset() {
        let items = [
            Desc58Item {
                country_code: *b"GBR",
                country_region_id: 0,
                local_time_offset_polarity: false,
                local_time_offset: 60,
                time_of_change: 1_798_000_000,
                next_time_offset: 0,
            },
            Desc58Item {
                country_code: *b"USA",
                country_region_id: 5,
                local_time_offset_polarity: true,
                local_time_offset: 300,
                time_of_change: 1_803_000_000,
                next_time_offset: 360,
            },
        ];

        let mut dst = Vec::new();
        Desc58 { items: &items }.encode(&mut dst).unwrap();

        let desc = Desc58Ref::try_from(first(&dst)).unwrap();
        let decoded: Vec<_> = desc.items().collect();
        assert_eq!(decoded, items);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x59, 0x00];
        assert!(Desc58Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x58, 0x03, 0x42, 0x55, 0x4c];
        assert!(Desc58Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn splits_oversized_list_into_repeated_descriptors() {
        let items: Vec<Desc58Item> = (0 .. 20)
            .map(|i| Desc58Item {
                country_code: *b"BUL",
                country_region_id: i,
                local_time_offset_polarity: false,
                local_time_offset: 120,
                time_of_change: 1_800_000_000,
                next_time_offset: 180,
            })
            .collect();

        let mut dst = Vec::new();
        Desc58 { items: &items }.encode(&mut dst).unwrap();

        let mut collected = Vec::new();
        for descriptor in DescriptorsRef::from(&dst[..]) {
            let desc = Desc58Ref::try_from(descriptor.unwrap()).unwrap();
            collected.push(desc.items().count());
        }
        assert_eq!(collected, [19, 1]);
    }
}
