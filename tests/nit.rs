extern crate mpegts;

mod data;
use mpegts::psi;


#[test]
fn test_parse_nit() {
    let mut psi = psi::Psi::default();
    psi.mux(&data::NIT_DVBS);
    assert!(psi.check());

    let mut nit = psi::Nit::default();
    nit.parse(&psi);

    assert_eq!(nit.table_id, 64);
    assert_eq!(nit.version, 11);
    assert_eq!(nit.network_id, 85);
    assert_eq!(nit.descriptors.len(), 0);
    assert_eq!(nit.items.len(), 6);

    let data: &[(u16, u16, usize)] = &[
        (8400, 318, 2),
        (12600, 318, 1),
        (700, 318, 1),
        (8900, 318, 1),
        (9400, 318, 1),
        (15600, 318, 1),
    ];

    for (item, (tsid, onid, desc_len)) in nit.items.iter().zip(data.iter()) {
        assert_eq!(item.tsid, *tsid);
        assert_eq!(item.onid, *onid);
        assert_eq!(item.descriptors.len(), *desc_len);
    }
}
