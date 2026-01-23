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
