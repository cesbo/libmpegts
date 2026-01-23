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

#[test]
fn test_assemble_tdt() {
    let mut tdt = Tdt::default();
    tdt.time = 1547057412;

    let mut cc: u8 = 3;
    let mut tdt_ts = Vec::<u8>::new();
    tdt.demux(TDT_PID, &mut cc, &mut tdt_ts);

    assert_eq!(data::TDT, tdt_ts.as_slice());
}
