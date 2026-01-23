mod data;

use mpegts::{
    psi::*,
    slicer::TsSlicer,
};

const SDT_DATA: &[(u16, u8, &str)] = &[
    /* PNR, EIT_schedule_flag, Service Type, Name */
    (1, 1, "Avalpa1: MPEG2 MHP"),
    (2, 1, "Avalpa2: MPEG2 MHEG5"),
    (3, 1, "Avalpa3: MPEG2 HBBTV"),
    (4, 1, "Avalpa4: MPEG2 TXT"),
    (5, 22, "Avalpa5: H264"),
    (6, 25, "Avalpa6: HD H264"),
];

#[test]
fn test_parse_sdt() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::SDT).for_each(|p| {
        psi.assemble(&p);
    });
    let sdt = SdtSectionRef::try_from(&psi).expect("Valid SDT section");

    assert_eq!(sdt.table_id(), 0x42);
    assert_eq!(sdt.version(), 1);
    assert_eq!(sdt.tsid(), 1);
    assert_eq!(sdt.onid(), 1);

    let mut count = 0;

    for (i, item) in sdt.items().enumerate() {
        let item = item.expect("Valid SDT item");
        let expected = SDT_DATA.get(i).expect("Expected SDT item");
        assert_eq!(item.pnr(), expected.0);
        assert_eq!(item.eit_schedule_flag(), false);
        assert_eq!(item.eit_present_following_flag(), true);
        assert_eq!(item.running_status(), 4);
        assert_eq!(item.free_ca_mode(), false);

        let mut descriptors = item.descriptors().expect("Service descriptors").into_iter();
        let desc = descriptors.next().expect("First service descriptor");
        assert_eq!(desc.tag(), 0x48); // Service Descriptor
        // assert_eq!(desc.service_type, expected.1);
        // assert_eq!(desc.provider.to_string(), "Avalpa");
        // assert_eq!(desc.name.to_string(), expected.2);

        assert!(descriptors.next().is_none());

        count += 1;
    }

    assert_eq!(count, SDT_DATA.len());
}
