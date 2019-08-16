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
    let mut reader = TsReader::new(io::BufReader::new(v.as_slice()));

    let mut buffer: [u8; ts::PACKET_SIZE] = [0; ts::PACKET_SIZE];

    let mut total = 0;
    loop {
        let x = reader.read(&mut buffer).unwrap();
        if x == 0 {
            break;
        }
        total += x;
    }
    assert_eq!(total, v.len());

    // TODO: check psi tables
}


#[test]
fn test_drain() {
    let mut v = Vec::with_capacity(188 * 10);
    v.extend_from_slice(data::PAT);
    v.extend_from_slice(data::PMT);
    v.extend_from_slice(data::SDT);
    let reader = TsReader::new(io::BufReader::new(v.as_slice()));
    let mut drain = TsDrain::new(reader);

    let mut o = Vec::with_capacity(188 * 10);
    io::copy(&mut drain, &mut o).unwrap();
    assert_eq!(v, o);
}


#[test]
fn test_drain_step()  {
    let mut v = Vec::with_capacity(188 * 10);
    v.extend_from_slice(data::PAT);
    v.extend_from_slice(data::PMT);
    v.extend_from_slice(data::SDT);
    let reader = TsReader::new(io::BufReader::new(v.as_slice()));
    let mut drain = TsDrain::new(reader);

    let mut o = Vec::with_capacity(188 * 10);

    let mut buf = unsafe {
        let mut v = Vec::with_capacity(32);
        v.set_len(32);
        v.into_boxed_slice()
    };

    use std::io::{Read, Write};
    loop {
        let len = drain.read(&mut buf).unwrap();
        if len == 0 { break }
        o.write_all(&buf[..len]).unwrap();
    }

    assert_eq!(v, o);
}
