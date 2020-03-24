use mpegts::{
    ts::TS,
    psi::*
};

mod data;


#[test]
fn test_parse_tdt() {
    let mut psi = Psi::default();
    let mut data_tdt = data::TDT.to_vec();
    let ts = TS::new(&mut data_tdt);
    psi.mux(ts);

    let mut tdt = Tdt::default();
    tdt.parse(&psi);

    assert_eq!(tdt.time, 1547057412);
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
