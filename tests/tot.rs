mod data;

use libmpegts::{
    psi::*,
    slicer::TsSlicer,
};

#[test]
fn test_parse_tot() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::TOT).for_each(|p| {
        psi.assemble(&p);
    });
    let tot = TotSectionRef::try_from(&psi).expect("Valid TOT section");

    assert_eq!(tot.time(), 1547057412);
}
