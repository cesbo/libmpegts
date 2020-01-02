use bitwrap::BitWrap;
use mpegts::{
    psi::*,
    es::*,
};
mod data;


#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    psi.mux(data::PMT);
    assert!(psi.check());

    let mut pmt = Pmt::default();
    pmt.unpack(&psi.buffer).unwrap();

    assert_eq!(pmt.version, 1);
    assert_eq!(pmt.pnr, 50455);
    assert_eq!(pmt.pcr, 2318);
    assert_eq!(pmt.descriptors.len(), 0);

    let item = &pmt.items[0];
    assert_eq!(item.stream_type, 2);
    assert_eq!(item.pid, 2318);
    let mut descriptors = item.descriptors.iter();
    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc0E(_v) => {}
        _ => unreachable!(),
    };

    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc09(_v) => {}
        _ => unreachable!(),
    };

    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc52(_v) => {}
        _ => unreachable!(),
    };

    let item = &pmt.items[1];
    assert_eq!(item.stream_type, 4);
    assert_eq!(item.pid, 2319);
    let mut descriptors = item.descriptors.iter();
    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc0E(_v) => {}
        _ => unreachable!(),
    };

    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc0A(_v) => {}
        _ => unreachable!(),
    };

    let desc = descriptors.next().unwrap();
    match desc {
        Descriptor::Desc52(_v) => {}
        _ => unreachable!(),
    };
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
        descriptors: Vec::default()
    };
    item.descriptors.push(Descriptor::Desc0E(Desc0E {
        bitrate: 77500
    }));
    item.descriptors.push(Descriptor::Desc09(Desc09 {
        caid: 2403,
        pid: 1281,
        data: Vec::new()
    }));
    item.descriptors.push(Descriptor::Desc52(Desc52 {
        tag: 1
    }));
    pmt.items.push(item);

    let mut item = PmtItem {
        stream_type: 4,
        pid: 2319,
        descriptors: Vec::default()
    };
    item.descriptors.push(Descriptor::Desc0E(Desc0E {
        bitrate: 77500
    }));
    item.descriptors.push(Descriptor::Desc0A(Desc0A {
        items: vec![
            Desc0Ai {
                code: *b"eng",
                audio_type: 1,
            },
        ]
    }));
    item.descriptors.push(Descriptor::Desc52(Desc52 {
        tag: 2
    }));
    pmt.items.push(item);

    let mut buffer: [u8; 1024] = [0; 1024];
    let result = pmt.pack(&mut buffer).unwrap();
    assert_eq!(&buffer[.. result], &data::PMT[5 .. result + 5]);
}


#[test]
fn test_pmt_get_stream_type() {
    let mut psi = Psi::default();
    psi.mux(data::PMT);

    let mut pmt = Pmt::default();
    pmt.unpack(&psi.buffer).unwrap();

    let mut iter = pmt.items.iter();
    assert_eq!(StreamType::VIDEO, iter.next().unwrap().get_stream_type());
    assert_eq!(StreamType::AUDIO, iter.next().unwrap().get_stream_type());
}
