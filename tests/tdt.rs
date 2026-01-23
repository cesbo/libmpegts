use mpegts::{
    psi::*,
    slicer::TsSlicer,
};
mod data;

#[test]
fn test_parse_tdt() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::TDT).for_each(|p| {
        psi.assemble(&p);
    });

    let payload = psi.payload().expect("TDT section payload expected");
    let tdt = TdtSectionRef::try_from(payload).expect("Valid TDT section");

    assert_eq!(tdt.time(), 1547057412);
}
