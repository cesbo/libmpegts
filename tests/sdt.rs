extern crate mpegts;

mod data;
use mpegts::psi;


#[test]
fn test_parse_sdt() {
    let mut psi = psi::Psi::default();
    
    let mut skip = 0;
    while skip < data::SDT.len() {
        psi.mux(&data::SDT[skip ..]);
        skip += 188;
    }
    assert!(psi.check());

    let mut sdt = psi::Sdt::default();
    sdt.parse(&psi);

    assert_eq!(sdt.table_id, 70);
    assert_eq!(sdt.version, 4);
    assert_eq!(sdt.tsid, 1);
    assert_eq!(sdt.onid, 112);
    assert_eq!(sdt.items.len(), 15);

    let item = sdt.items.iter().next().unwrap();
    assert_eq!(item.pnr, 605);
    assert_eq!(item.eit_schedule_flag, 0);
    assert_eq!(item.eit_present_following_flag, 0);
    assert_eq!(item.running_status, 1);
    assert_eq!(item.free_ca_mode, 0);
    assert_eq!(item.descriptors.len(), 1);

    let desc =  match item.descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc48(v) => v,
        _ => unreachable!(),
    };
    assert_eq!(desc.type_, 12);
    assert_eq!(desc.provider.to_string(), "HTB+");
    assert_eq!(desc.name.to_string(), "Neotion Update Service");
}
