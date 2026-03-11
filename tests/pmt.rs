mod data;

use libmpegts::{
    psi::*,
    slicer::TsSlicer,
};

#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::PMT).for_each(|p| {
        psi.assemble(&p);
    });
    let pmt = PmtSectionRef::try_from(&psi).expect("Valid PMT section");

    assert_eq!(pmt.version(), 1);
    assert_eq!(pmt.program_number(), 50455);
    assert_eq!(pmt.pcr_pid(), 2318);

    let mut streams = pmt.streams();

    let stream = streams
        .next()
        .expect("First stream")
        .expect("Valid PMT stream");
    assert_eq!(stream.stream_type(), 2);
    assert_eq!(stream.elementary_pid(), 2318);
    let mut descriptors = stream
        .stream_descriptors()
        .expect("Video descriptors")
        .into_iter();
    let desc = descriptors
        .next()
        .expect("First video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors
        .next()
        .expect("Second video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x09);
    let desc = descriptors
        .next()
        .expect("Third video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x52);

    let stream = streams
        .next()
        .expect("Second stream")
        .expect("Valid PMT stream");
    assert_eq!(stream.stream_type(), 4);
    assert_eq!(stream.elementary_pid(), 2319);
    let mut descriptors = stream
        .stream_descriptors()
        .expect("Audio descriptors")
        .into_iter();
    let desc = descriptors
        .next()
        .expect("First audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors
        .next()
        .expect("Second audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0A);
    let desc = descriptors
        .next()
        .expect("Third audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x52);
}

#[test]
fn test_build_pmt_roundtrip() {
    let es1_descriptors: &[u8] = &[
        0x0e, 0x03, 0xc1, 0x2e, 0xbc, 0x09, 0x04, 0x09, 0x63, 0xe5, 0x01, 0x52, 0x01, 0x01,
    ];
    let es2_descriptors: &[u8] = &[
        0x0e, 0x03, 0xc1, 0x2e, 0xbc, 0x0a, 0x04, 0x65, 0x6e, 0x67, 0x01, 0x52, 0x01, 0x02,
    ];

    let sections = PmtBuilder::build(PmtConfig {
        program_number: 50455,
        pcr_pid: 2318,
        version: 1,
        program_descriptors: Vec::new(),
        streams: vec![
            PmtStream {
                stream_type: 2,
                elementary_pid: 2318,
                stream_descriptors: es1_descriptors.to_vec(),
            },
            PmtStream {
                stream_type: 4,
                elementary_pid: 2319,
                stream_descriptors: es2_descriptors.to_vec(),
            },
        ],
    });

    assert_eq!(sections.len(), 1);

    let expected = &data::PMT[5 .. 59];
    assert_eq!(&sections[0], expected);
}

#[test]
fn test_build_pmt_empty() {
    let sections = PmtBuilder::build(PmtConfig {
        program_number: 1,
        pcr_pid: 256,
        version: 0,
        program_descriptors: Vec::new(),
        streams: Vec::new(),
    });

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].len(), 16); // header(12) + CRC(4)

    let pmt = PmtSectionRef::try_from(&sections[0][..]).expect("Valid empty PMT");
    assert_eq!(pmt.program_number(), 1);
    assert_eq!(pmt.pcr_pid(), 256);
    assert_eq!(pmt.version(), 0);
    assert!(pmt.program_descriptors().is_none());
    assert_eq!(pmt.streams().count(), 0);
}

#[test]
fn test_build_pmt_with_descriptors() {
    let program_descriptors: &[u8] = &[0x09, 0x04, 0x01, 0x02, 0x03, 0x04];

    let sections = PmtBuilder::build(PmtConfig {
        program_number: 100,
        pcr_pid: 512,
        version: 3,
        program_descriptors: program_descriptors.to_vec(),
        streams: vec![PmtStream {
            stream_type: 2,
            elementary_pid: 513,
            stream_descriptors: Vec::new(),
        }],
    });

    assert_eq!(sections.len(), 1);

    let pmt = PmtSectionRef::try_from(&sections[0][..]).expect("Valid PMT");
    assert_eq!(pmt.program_number(), 100);
    assert_eq!(pmt.pcr_pid(), 512);
    assert_eq!(pmt.version(), 3);

    let descriptors = pmt.program_descriptors().expect("Program descriptors");
    let desc = descriptors.into_iter().next().unwrap().unwrap();
    assert_eq!(desc.tag(), 0x09);

    let mut streams = pmt.streams();
    let stream = streams.next().expect("First stream").expect("Valid stream");
    assert_eq!(stream.stream_type(), 2);
    assert_eq!(stream.elementary_pid(), 513);
    assert!(stream.stream_descriptors().is_none());
    assert!(streams.next().is_none());
}

#[test]
fn test_build_pmt_sections_index() {
    let sections = PmtBuilder::build(PmtConfig {
        program_number: 1,
        pcr_pid: 100,
        version: 0,
        program_descriptors: Vec::new(),
        streams: vec![PmtStream {
            stream_type: 2,
            elementary_pid: 101,
            stream_descriptors: Vec::new(),
        }],
    });

    assert_eq!(sections.len(), 1);
    PmtSectionRef::try_from(&sections[0][..]).expect("Valid section");
}
