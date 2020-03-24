use mpegts::{
    ts::TS,
    psi::*
};

mod data;

#[test]
fn test_parse_tot() {
    let mut psi = Psi::default();
    let mut data_tot = data::TOT.to_vec();
    let ts = TS::new(&mut data_tot);
    psi.mux(ts);

    let mut tot = Tot::default();
    tot.parse(&psi);

    assert_eq!(tot.time, 1547057412);
}

#[test]
fn test_assemble_tot() {
    let mut tot = Tot::default();
    tot.time = 1547057412;
    tot.descriptors.push(DescRaw {
        tag: 0x9a,
        data: vec![0xe4, 0xb8, 0x02, 0x00, 0x00, 0xe5, 0xa6, 0x02, 0x00, 0x00],
    });

    let mut cc: u8 = 4;
    let mut tot_ts = Vec::<u8>::new();
    tot.demux(TOT_PID, &mut cc, &mut tot_ts);

    assert_eq!(data::TOT, tot_ts.as_slice());
}
