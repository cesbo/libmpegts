use mpegts::{
    ts::TS,
    psi::*
};

mod data;


#[test]
fn test_parse_pat() {
    let mut psi = Psi::default();
    let mut data_pat = data::PAT.to_vec();
    let ts = TS::new(&mut data_pat);
    psi.mux(ts);
    assert!(psi.check());

    let pat = Pat::from(&psi);

    assert_eq!(pat.version, 1);
    assert_eq!(pat.tsid, 1);
    assert_eq!(pat.items.len(), 7);
    for item in pat.items.iter() {
        match item.pnr {
            0 => assert_eq!(item.pid, 16),
            1 => assert_eq!(item.pid, 1031),
            2 => assert_eq!(item.pid, 1032),
            3 => assert_eq!(item.pid, 1033),
            4 => assert_eq!(item.pid, 1034),
            5 => assert_eq!(item.pid, 1035),
            6 => assert_eq!(item.pid, 1036),
            _ => unreachable!(),
        };
    }
}


#[test]
fn test_assemble_pat() {
    let mut pat = Pat::default();
    pat.version = 1;
    pat.tsid = 1;
    pat.items.push(PatItem { pnr: 0, pid: 16 });
    pat.items.push(PatItem { pnr: 1, pid: 1031 });
    pat.items.push(PatItem { pnr: 2, pid: 1032 });
    pat.items.push(PatItem { pnr: 3, pid: 1033 });
    pat.items.push(PatItem { pnr: 4, pid: 1034 });
    pat.items.push(PatItem { pnr: 5, pid: 1035 });
    pat.items.push(PatItem { pnr: 6, pid: 1036 });

    let mut cc: u8 = 0;
    let mut pat_ts = Vec::<u8>::new();
    pat.demux(PAT_PID, &mut cc, &mut pat_ts);

    assert_eq!(data::PAT, pat_ts.as_slice());
}
