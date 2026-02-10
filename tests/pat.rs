mod data;

use mpegts::{
    psi::*,
    slicer::TsSlicer,
};

#[test]
fn test_parse_pat() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::PAT).for_each(|p| {
        psi.assemble(&p);
    });
    let pat = PatSectionRef::try_from(&psi).expect("Valid PAT section");

    assert_eq!(pat.version(), 1);
    assert_eq!(pat.tsid(), 1);

    let mut count = 0;
    for item in pat.items() {
        let item = item.expect("Valid PAT item");
        match item.pnr() {
            0 => assert_eq!(item.pid(), 16),
            1 => assert_eq!(item.pid(), 1031),
            2 => assert_eq!(item.pid(), 1032),
            3 => assert_eq!(item.pid(), 1033),
            4 => assert_eq!(item.pid(), 1034),
            5 => assert_eq!(item.pid(), 1035),
            6 => assert_eq!(item.pid(), 1036),
            _ => unreachable!(),
        };
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn test_build_pat_roundtrip() {
    let mut builder = PatBuilder::new(1);
    builder.set_version(1);
    builder.push(0, 16);
    builder.push(1, 1031);
    builder.push(2, 1032);
    builder.push(3, 1033);
    builder.push(4, 1034);
    builder.push(5, 1035);
    builder.push(6, 1036);
    let sections = builder.finalize();

    assert_eq!(sections.len(), 1);

    let expected = &data::PAT[5 .. 45];
    assert_eq!(&sections[0], expected);
}

#[test]
fn test_build_pat_empty() {
    let builder = PatBuilder::new(42);
    let sections = builder.finalize();

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].len(), 12); // header(8) + CRC(4)

    let pat = PatSectionRef::try_from(&sections[0][..]).expect("Valid empty PAT");
    assert_eq!(pat.tsid(), 42);
    assert_eq!(pat.version(), 0);
    assert_eq!(pat.items().count(), 0);
}

#[test]
fn test_build_pat_sections_iter() {
    let mut builder = PatBuilder::new(1);
    builder.push(1, 100);
    let sections = builder.finalize();

    let mut count = 0;
    for section in &sections {
        PatSectionRef::try_from(section).expect("Valid section");
        count += 1;
    }
    assert_eq!(count, 1);
}
