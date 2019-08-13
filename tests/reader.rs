use std::io::{
    self,
    BufReader,
};
use mpegts::reader::*;
mod data;


#[test]
fn test_reader() {
    let mut v = Vec::with_capacity(188 * 10);
    v.extend_from_slice(data::PAT);
    v.extend_from_slice(data::PMT);
    v.extend_from_slice(data::SDT);
    let reader = BufReader::new(v.as_slice());
    let mut tsreader = TsReader::new(reader);
    let r = io::copy(&mut tsreader, &mut io::sink()).unwrap();
    println!("r={}", r);
}
