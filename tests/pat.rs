use bitwrap::BitWrap;
use mpegts::psi::*;
mod data;

#[test]
fn test_parse_pat() {
    let mut psi = Psi::default();
    psi.mux(data::PAT);
    assert!(psi.check());

    let mut pat = Pat::default();
    pat.unpack(&psi.buffer).unwrap();

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

    let mut buffer: [u8; 1024] = [0; 1024];
    let result = pat.pack(&mut buffer).unwrap();
    assert_eq!(&buffer[.. result - 4], &data::PAT[5 .. result + 5 - 4]);
}
