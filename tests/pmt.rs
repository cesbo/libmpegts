extern crate mpegts;

mod data;
use mpegts::{psi, textcode};


#[test]
fn test_parse_pmt() {
    let mut psi = psi::Psi::default();
    psi.mux(&data::PMT);
    assert!(psi.check());

    let mut pmt = psi::Pmt::default();
    pmt.parse(&psi);

    assert_eq!(pmt.version, 1);
    assert_eq!(pmt.pnr, 50455);
    assert_eq!(pmt.pcr, 2318);
    assert_eq!(pmt.descriptors.len(), 0);

    let item = &pmt.items[0];
    assert_eq!(item.stream_type, 2);
    assert_eq!(item.pid, 2318);
    let mut descriptors = item.descriptors.iter();
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc09(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };

    let item = &pmt.items[1];
    assert_eq!(item.stream_type, 4);
    assert_eq!(item.pid, 2319);
    let mut descriptors = item.descriptors.iter();
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc0A(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        psi::Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };
}

#[test]
fn test_assemble_pmt() {
    let mut pmt = psi::Pmt::default();
    pmt.version = 1;
    pmt.pnr = 50455;
    pmt.pcr = 2318;

    let mut item = psi::PmtItem {
        stream_type: 2,
        pid: 2318,
        descriptors: psi::Descriptors::default()
    };
    item.descriptors.push(
        psi::Descriptor::Desc09(
            psi::Desc09 {
                caid: 2403,
                pid: 1281,
                data: Vec::new()
            }
        )
    );
    item.descriptors.push(
        psi::Descriptor::Desc0E(
            psi::Desc0E {
                bitrate: 77500
            }
        )
    );
    item.descriptors.push(
        psi::Descriptor::Desc52(
            psi::Desc52 {
                tag: 1
            }
        )
    );
    pmt.items.push(item);

    let mut item = psi::PmtItem {
        stream_type: 4,
        pid: 2319,
        descriptors: psi::Descriptors::default()
    };
    item.descriptors.push(
        psi::Descriptor::Desc0A(
            psi::Desc0A {
                languages: vec!(
                    psi::Language {
                        code: textcode::StringDVB::from_str("eng", 0),
                        audio_type: 1
                    }
                )
            }
        )
    );
    item.descriptors.push(
        psi::Descriptor::Desc52(
            psi::Desc52 {
                tag: 2
            }
        )
    );
    pmt.items.push(item);

    let mut psi_assembled = psi::Psi::default();
    pmt.assemble(&mut psi_assembled);
    psi_assembled.finalize();

    let mut psi_check = psi::Psi::default();
    let mut skip = 0;
    while skip < data::PMT.len() {
        psi_check.mux(&data::PMT[skip ..]);
        skip += 188;
    }

    assert_eq!(psi_assembled, psi_check);
    assert_eq!(&psi_assembled.buffer[.. psi_assembled.size], &psi_check.buffer[.. psi_check.size]);
}
