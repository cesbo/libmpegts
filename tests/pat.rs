mod data;

use libmpegts::{
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
    let sections = PatBuilder::build(PatConfig {
        tsid: 1,
        version: 1,
        programs: vec![
            PatProgram { pnr: 0, pid: 16 },
            PatProgram { pnr: 1, pid: 1031 },
            PatProgram { pnr: 2, pid: 1032 },
            PatProgram { pnr: 3, pid: 1033 },
            PatProgram { pnr: 4, pid: 1034 },
            PatProgram { pnr: 5, pid: 1035 },
            PatProgram { pnr: 6, pid: 1036 },
        ],
    });

    assert_eq!(sections.len(), 1);

    let expected = &data::PAT[5 .. 45];
    assert_eq!(&sections[0], expected);
}

#[test]
fn test_build_pat_empty() {
    let sections = PatBuilder::build(PatConfig {
        tsid: 42,
        version: 0,
        programs: Vec::new(),
    });

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].len(), 12); // header(8) + CRC(4)

    let pat = PatSectionRef::try_from(&sections[0][..]).expect("Valid empty PAT");
    assert_eq!(pat.tsid(), 42);
    assert_eq!(pat.version(), 0);
    assert_eq!(pat.items().count(), 0);
}

#[test]
fn test_build_pat_sections_index() {
    let sections = PatBuilder::build(PatConfig {
        tsid: 1,
        version: 0,
        programs: vec![PatProgram { pnr: 1, pid: 100 }],
    });

    assert_eq!(sections.len(), 1);
    PatSectionRef::try_from(&sections[0][..]).expect("Valid section");
}
