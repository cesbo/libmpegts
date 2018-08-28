extern crate mpegts;
use mpegts::psi::*;
use mpegts::textcode::*;

mod data;
use data::*;

#[test]
fn test_parse_eit_4e() {
    let mut psi = Psi::default();
    psi.mux(&EIT_4E);
    assert!(psi.check());

    let mut eit = Eit::default();
    eit.parse(&psi);

    assert_eq!(eit.version, 1);
    assert_eq!(eit.pnr, 6);
    assert_eq!(eit.tsid, 1);
    assert_eq!(eit.onid, 1);

    assert_eq!(eit.items.len(), 1);
    let item = eit.items.iter().next().unwrap();
    assert_eq!(item.event_id, 1);
    assert_eq!(item.start, 1296432000);
    assert_eq!(item.duration, 72000);
    assert_eq!(item.status, 4);
    assert_eq!(item.ca_mode, 0);
}

#[test]
fn test_assemble_eit_4e() {
    let mut eit = Eit::default();
    eit.table_id = 0x4E;
    eit.version = 1;
    eit.pnr = 6;
    eit.tsid = 1;
    eit.onid = 1;

    let mut item = EitItem::default();
    item.event_id = 1;
    item.start = 1296432000;
    item.duration = 72000;
    item.status = 4;
    item.ca_mode = 0;
    item.descriptors.push(Descriptor::Desc4D(Desc4D {
        lang: StringDVB::from_str(0, "ita"),
        name: StringDVB::from_str(0, "H264 HD 1080 24p"),
        text: StringDVB::from_str(0, "elementary video bit rate is 7.2Mbps, audio ac3 5.1, note: 24p is not currently/officially supported by DVB standards"),
    }));

    eit.items.push(item);

    let mut psi_custom = Psi::default();
    eit.assemble(&mut psi_custom);

    let mut psi_check = Psi::default();
    psi_check.mux(&EIT_4E);

    assert_eq!(psi_custom, psi_check);
    assert_eq!(&psi_custom.buffer[.. psi_custom.size], &psi_check.buffer[.. psi_check.size]);
}

#[test]
fn test_parse_eit_50() {
    let mut psi = Psi::default();

    let mut skip = 0;
    while skip < EIT_50.len() {
        psi.mux(&EIT_50[skip ..]);
        skip += 188;
    }
    assert!(psi.check());

    let mut eit = Eit::default();
    eit.parse(&psi);

    assert_eq!(eit.version, 21);
    assert_eq!(eit.pnr, 7375);
    assert_eq!(eit.tsid, 7400);
    assert_eq!(eit.onid, 1);

    assert_eq!(eit.items.len(), 1);
    let item = eit.items.iter().next().unwrap();
    assert_eq!(item.event_id, 31948);
    assert_eq!(item.start, 1534183800);
    assert_eq!(item.duration, 1800);
    assert_eq!(item.status, 0);
    assert_eq!(item.ca_mode, 1);

    assert_eq!(item.descriptors.len(), 4);
}

#[test]
fn test_assemble_eit_50() {
    let mut eit = Eit::default();
    eit.table_id = 0x50;
    eit.version = 21;
    eit.pnr = 7375;
    eit.tsid = 7400;
    eit.onid = 1;

    let mut item = EitItem::default();
    item.event_id = 31948;
    item.start = 1534183800;
    item.duration = 1800;
    item.status = 0;
    item.ca_mode = 1;

    item.descriptors.push(Descriptor::Desc4D(Desc4D {
        lang: StringDVB::from_str(0, "pol"),
        name: StringDVB::from_str(2, "Ostatni prawdziwy mężczyzna 4: odc.5"),
        text: StringDVB::from_str(0, ""),
    }));

    item.descriptors.push(Descriptor::Desc4E(Desc4E {
        number: 0,
        last_number: 1,
        lang: StringDVB::from_str(0, "pol"),
        items: Vec::new(),
        text: StringDVB::from_str(2, "serial komediowy (USA, 2014) odc.5, Szkolna fuzja?Występują: Tim Allen, Nancy Travis, Molly Ephraim?Mike i Chuck debatują na temat zalet lokalnego referendum o połączeniu ich ekskluzywnej szkoły średniej z sąsiedztwa z placówką ze śródmieścia. Z"),
    }));

    item.descriptors.push(Descriptor::Desc4E(Desc4E {
        number: 1,
        last_number: 1,
        lang: StringDVB::from_str(0, "pol"),
        items: Vec::new(),
        text: StringDVB::from_str(2, " okazji Halloween, Ryan przebiera Boyda za bryłę węgla. Ma to być kolejnym przypomnieniem dla Vanessy, że jej praca jako geologa może szkodzić środowisku naturalnemu.?Reżyser: John Pasquin?Od lat: 12"),
    }));

    item.descriptors.push(Descriptor::DescRaw(DescRaw {
        tag: 0x55,
        data: vec![80, 76, 32, 9],
    }));

    eit.items.push(item);

    let mut psi_custom = Psi::default();
    eit.assemble(&mut psi_custom);

    let mut psi_check = Psi::default();
    let mut skip = 0;
    while skip < EIT_50.len() {
        psi_check.mux(&EIT_50[skip ..]);
        skip += 188;
    }

    assert_eq!(psi_custom, psi_check);
    assert_eq!(&psi_custom.buffer[.. psi_custom.size], &psi_check.buffer[.. psi_check.size]);
}
