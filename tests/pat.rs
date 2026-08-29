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
    let section = psi.sections().first().expect("PAT section expected");
    let pat = PatSectionRef::try_from(section).expect("Valid PAT section");

    assert_eq!(pat.version(), 1);
    assert_eq!(pat.transport_stream_id(), 1);

    let mut count = 0;
    for program in pat.programs() {
        let program = program.expect("Valid PAT program");
        match program.program_number() {
            0 => assert_eq!(program.pid(), 16),
            1 => assert_eq!(program.pid(), 1031),
            2 => assert_eq!(program.pid(), 1032),
            3 => assert_eq!(program.pid(), 1033),
            4 => assert_eq!(program.pid(), 1034),
            5 => assert_eq!(program.pid(), 1035),
            6 => assert_eq!(program.pid(), 1036),
            _ => unreachable!(),
        };
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn test_build_pat_roundtrip() {
    let sections = PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 1,
        programs: vec![
            PatProgram {
                program_number: 0,
                pid: 16,
            },
            PatProgram {
                program_number: 1,
                pid: 1031,
            },
            PatProgram {
                program_number: 2,
                pid: 1032,
            },
            PatProgram {
                program_number: 3,
                pid: 1033,
            },
            PatProgram {
                program_number: 4,
                pid: 1034,
            },
            PatProgram {
                program_number: 5,
                pid: 1035,
            },
            PatProgram {
                program_number: 6,
                pid: 1036,
            },
        ],
    });

    assert_eq!(sections.len(), 1);

    let expected = &data::PAT[5 .. 45];
    assert_eq!(&sections[0], expected);
}

#[test]
fn test_build_pat_empty() {
    let sections = PatBuilder::build(PatConfig {
        transport_stream_id: 42,
        version: 0,
        programs: Vec::new(),
    });

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].len(), 12); // header(8) + CRC(4)

    let pat = PatSectionRef::try_from(&sections[0][..]).expect("Valid empty PAT");
    assert_eq!(pat.transport_stream_id(), 42);
    assert_eq!(pat.version(), 0);
    assert_eq!(pat.programs().count(), 0);
}

#[test]
fn test_build_pat_sections_index() {
    let sections = PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 0,
        programs: vec![PatProgram {
            program_number: 1,
            pid: 100,
        }],
    });

    assert_eq!(sections.len(), 1);
    PatSectionRef::try_from(&sections[0][..]).expect("Valid section");
}
