extern crate mpegts;
use mpegts::psi::*;
use mpegts::textcode::*;

mod data;

const EIT_4E_LANG: &str = "ita";
const EIT_4E_NAME: &str = "H264 HD 1080 24p";
const EIT_4E_TEXT: &str = "elementary video bit rate is 7.2Mbps, audio ac3 5.1, note: 24p is not currently/officially supported by DVB standards";

#[test]
fn test_parse_eit_4e() {
    let mut psi = Psi::default();
    psi.mux(&data::EIT_4E);
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
    assert_eq!(item.descriptors.len(), 1);
    let desc = match item.descriptors.iter().next().unwrap() {
        Descriptor::Desc4D(v) => v,
        _ => unreachable!(),
    };
    assert_eq!(&desc.lang.to_string(), EIT_4E_LANG);
    assert_eq!(&desc.name.to_string(), EIT_4E_NAME);
    assert_eq!(&desc.text.to_string(), EIT_4E_TEXT);
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
        lang: StringDVB::from_str(EIT_4E_LANG, ISO6937),
        name: StringDVB::from_str(EIT_4E_NAME, ISO6937),
        text: StringDVB::from_str(EIT_4E_TEXT, ISO6937),
    }));

    eit.items.push(item);

    let mut cc: u8 = 0;
    let mut eit_4e_ts = Vec::<u8>::new();
    eit.demux(EIT_PID, &mut cc, &mut eit_4e_ts);

    assert_eq!(data::EIT_4E, eit_4e_ts.as_slice());
}

#[test]
fn test_parse_eit_50() {
    let mut psi = Psi::default();

    let mut skip = 0;
    while skip < data::EIT_50.len() {
        psi.mux(&data::EIT_50[skip ..]);
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
        lang: StringDVB::from_str("pol", 0),
        name: StringDVB::from_str("Ostatni prawdziwy mężczyzna 4: odc.5", 2),
        text: StringDVB::from_str("", 2),
    }));

    item.descriptors.push(Descriptor::Desc4E(Desc4E {
        number: 0,
        last_number: 1,
        lang: StringDVB::from_str("pol", 0),
        items: Vec::new(),
        text: StringDVB::from_str("serial komediowy (USA, 2014) odc.5, Szkolna fuzja?Występują: Tim Allen, Nancy Travis, Molly Ephraim?Mike i Chuck debatują na temat zalet lokalnego referendum o połączeniu ich ekskluzywnej szkoły średniej z sąsiedztwa z placówką ze śródmieścia. Z", 2),
    }));

    item.descriptors.push(Descriptor::Desc4E(Desc4E {
        number: 1,
        last_number: 1,
        lang: StringDVB::from_str("pol", 0),
        items: Vec::new(),
        text: StringDVB::from_str(" okazji Halloween, Ryan przebiera Boyda za bryłę węgla. Ma to być kolejnym przypomnieniem dla Vanessy, że jej praca jako geologa może szkodzić środowisku naturalnemu.?Reżyser: John Pasquin?Od lat: 12", 2),
    }));

    item.descriptors.push(Descriptor::DescRaw(DescRaw {
        tag: 0x55,
        data: vec![80, 76, 32, 9],
    }));

    eit.items.push(item);

    let mut cc: u8 = 4;
    let mut eit_50_ts = Vec::<u8>::new();
    eit.demux(EIT_PID, &mut cc, &mut eit_50_ts);

    assert_eq!(data::EIT_50, eit_50_ts.as_slice());
}
