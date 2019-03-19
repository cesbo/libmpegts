use mpegts::psi::*;
use mpegts::textcode::*;
mod data;

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
    let mut skip = 0;
    while skip < data::SDT.len() {
        psi.mux(&data::SDT[skip ..]);
        skip += 188;
    }
    assert!(psi.check());

    let mut sdt = Sdt::default();
    sdt.parse(&psi);

    assert_eq!(sdt.table_id, 0x42);
    assert_eq!(sdt.version, 1);
    assert_eq!(sdt.tsid, 1);
    assert_eq!(sdt.onid, 1);
    assert_eq!(sdt.items.len(), 6);

    let mut items = sdt.items.iter();
    for d in SDT_DATA {
        let item = items.next().unwrap();
        assert_eq!(item.pnr, d.0);
        assert_eq!(item.eit_schedule_flag, 0);
        assert_eq!(item.eit_present_following_flag, 1);
        assert_eq!(item.running_status, 4);
        assert_eq!(item.free_ca_mode, 0);
        assert_eq!(item.descriptors.len(), 1);

        let desc = item.descriptors.iter().next().unwrap().inner::<Desc48>();
        assert_eq!(desc.service_type, d.1);
        assert_eq!(desc.provider.to_string(), "Avalpa");
        assert_eq!(desc.name.to_string(), d.2);
    }
}

#[test]
fn test_assemble_sdt() {
    let mut sdt = Sdt::default();
    sdt.table_id = 0x42;
    sdt.version = 1;
    sdt.tsid = 1;
    sdt.onid = 1;

    for d in SDT_DATA {
        let mut item = SdtItem::default();
        item.pnr = d.0;
        item.eit_schedule_flag = 0;
        item.eit_present_following_flag = 1;
        item.running_status = 4;
        item.free_ca_mode = 0;

        item.descriptors.push(Desc48 {
            service_type: d.1,
            provider: StringDVB::from_str("Avalpa", ISO6937),
            name: StringDVB::from_str(d.2, ISO6937)
        });

        sdt.items.push(item);
    }

    let mut cc: u8 = 10;
    let mut sdt_ts = Vec::<u8>::new();
    sdt.demux(SDT_PID, &mut cc, &mut sdt_ts);

    assert_eq!(data::SDT, sdt_ts.as_slice());
}
