use std::{
    io,
};
use mpegts::{
    ts,
    reader::*,
};
mod data;


#[test]
fn test_reader() {
    let mut v = Vec::with_capacity(188 * 10);
    v.extend_from_slice(data::PAT);
    v.extend_from_slice(data::PMT);
    v.extend_from_slice(data::SDT);
    let reader = io::BufReader::new(v.as_slice());
    let mut tsreader = TsReader::new(reader);

    let mut buffer: [u8; ts::PACKET_SIZE] = [0; ts::PACKET_SIZE];

    let mut total = 0;
    loop {
        let x = tsreader.read(&mut buffer).unwrap();
        if x == 0 {
            break;
        }
        total += x;

        dbg!(&ts::TsPacket::new(&buffer));
    }
    assert_eq!(total, v.len());

    // TODO: check psi tables
}
