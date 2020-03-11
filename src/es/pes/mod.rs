// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU
pub mod peshead;
pub use self::peshead::*;


use std::io::{
    self,
    BufRead
};


#[derive(Debug, Error)]
pub enum PESError {
    #[error_from]
    Io(io::Error),
}


pub type Result<T> = std::result::Result<T, PESError>;


pub struct PES {
    head: PESHead,
    data: [u8; 156],
}


impl PES {
    pub fn new(reader: &mut dyn BufRead) -> Result<Self> {
        let mut data =[0; 156];
        reader.read_exact(&mut data)?;

        Ok(Self {
            head: PESHead::default(),
            data
        })
    }
}


/// PTS - Presentation Timestamp
/// 90clocks = 1ms
pub const PTS_CLOCK_MS: u64 = 90;
pub const PTS_NONE: u64 = 1 << 33;
pub const PTS_MAX: u64 = PTS_NONE - 1;


/// Returns `true` if packet has valid prefix
#[inline]
pub fn is_prefix(packet: &[u8]) -> bool {
    packet[0] == 0x00 && packet[1] == 0x00 && packet[2] == 0x01
}


/// According to Table 2-17 in ISO-13818-1
#[inline]
pub fn is_syntax_spec(packet: &[u8]) -> bool {
    match packet[3] {
        0xBC => false,  // program_stream_map
        0xBE => false,  // padding_stream
        0xBF => false,  // private_stream_2
        0xF0 => false,  // ECM
        0xF1 => false,  // EMM
        0xF2 => false,  // DSMCC_stream
        0xF8 => false,  // ITU-T Rec. H.222.1 type E
        0xFF => false,  // program_stream_directory
        _ => true,
    }
}


/// Returns `true` if PTS bit is set in the PTS_DTS_flags
#[inline]
pub fn is_pts(packet: &[u8]) -> bool {
    (packet[7] & 0x80) != 0
}


/// Returns PTS value
#[inline]
pub fn get_pts(packet: &[u8]) -> u64 {
    (u64::from(packet[ 9] & 0x0E) << 29) |
    (u64::from(packet[10]       ) << 22) |
    (u64::from(packet[11] & 0xFE) << 14) |
    (u64::from(packet[12]       ) <<  7) |
    (u64::from(packet[13]       ) >>  1)
}


/// Returns difference between previous PTS and current PTS
#[inline]
pub fn pts_delta(last_pts: u64, current_pts: u64) -> u64 {
    if current_pts >= last_pts {
        current_pts - last_pts
    } else {
        current_pts + PTS_MAX - last_pts
    }
}


/// Converts PTS to milliseconds
#[inline]
pub fn pts_to_ms(pts: u64) -> u64 { pts / PTS_CLOCK_MS }


/// Returns `true` if DTS bit is set in the PTS_DTS_flags
#[inline]
pub fn is_dts(packet: &[u8]) -> bool {
    (packet[7] & 0x40) != 0
}


/// Returns DTS value
#[inline]
pub fn get_dts(packet: &[u8]) -> u64 {
    (u64::from(packet[14] & 0x0E) << 29) |
    (u64::from(packet[15]       ) << 22) |
    (u64::from(packet[16] & 0xFE) << 14) |
    (u64::from(packet[17]       ) <<  7) |
    (u64::from(packet[18]       ) >>  1)
}
