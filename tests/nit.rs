mod data;

use libmpegts::{
    psi::*,
    slicer::TsSlicer,
};

#[test]
fn test_parse_nit() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::NIT_DVBS).for_each(|p| {
        psi.assemble(&p);
    });
    let nit = NitSectionRef::try_from(&psi).expect("Valid NIT section");

    assert_eq!(nit.table_id(), 0x40);
    assert_eq!(nit.version(), 11);
    assert_eq!(nit.network_id(), 85);

    assert!(nit.descriptors().is_none());

    const NIT_DATA: &[(u16, u16, usize)] = &[
        (8400, 318, 2),
        (12600, 318, 1),
        (700, 318, 1),
        (8900, 318, 1),
        (9400, 318, 1),
        (15600, 318, 1),
    ];

    for (i, item) in nit.items().enumerate() {
        let item = item.expect("Valid NIT item");
        let expected = NIT_DATA.get(i).expect("Expected NIT item");

        assert_eq!(item.tsid(), expected.0);
        assert_eq!(item.onid(), expected.1);

        let descriptors = item
            .descriptors()
            .expect("Service descriptors")
            .into_iter()
            .filter_map(Result::ok)
            .count();
        assert_eq!(descriptors, expected.2);
    }
}
