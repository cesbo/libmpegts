use mpegts::{
    psi::*,
    textcode::*,
};
mod data;


#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    psi.mux(data::PMT);
    assert!(psi.check());

    let mut pmt = Pmt::default();
    pmt.parse(&psi);

    assert_eq!(pmt.version, 1);
    assert_eq!(pmt.pnr, 50455);
    assert_eq!(pmt.pcr, 2318);
    assert_eq!(pmt.descriptors.len(), 0);

    let item = &pmt.items[0];
    assert_eq!(item.stream_type, 2);
    assert_eq!(item.pid, 2318);
    let mut descriptors = item.descriptors.iter();
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x09);
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x52);

    let item = &pmt.items[1];
    assert_eq!(item.stream_type, 4);
    assert_eq!(item.pid, 2319);
    let mut descriptors = item.descriptors.iter();
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x0A);
    let desc = descriptors.next().unwrap();
    assert_eq!(desc.tag(), 0x52);
}


#[test]
fn test_assemble_pmt() {
    let mut pmt = Pmt::default();
    pmt.version = 1;
    pmt.pnr = 50455;
    pmt.pcr = 2318;

    let mut item = PmtItem {
        stream_type: 2,
        pid: 2318,
        descriptors: Descriptors::default()
    };
    item.descriptors.push(Desc0E {
        bitrate: 77500
    });
    item.descriptors.push(Desc09 {
        caid: 2403,
        pid: 1281,
        data: Vec::new()
    });
    item.descriptors.push(Desc52 {
        tag: 1
    });
    pmt.items.push(item);

    let mut item = PmtItem {
        stream_type: 4,
        pid: 2319,
        descriptors: Descriptors::default()
    };
    item.descriptors.push(Desc0E {
        bitrate: 77500
    });
    item.descriptors.push(Desc0A {
        items: vec![
            Desc0Ai {
                code: StringDVB::from_str("eng", ISO6937),
                audio_type: 1,
            },
        ]
    });
    item.descriptors.push(Desc52 {
        tag: 2
    });
    pmt.items.push(item);

    let pid = 278;
    let mut cc: u8 = 0;
    let mut pmt_ts = Vec::<u8>::new();
    pmt.demux(pid, &mut cc, &mut pmt_ts);

    assert_eq!(data::PMT, pmt_ts.as_slice());
}
