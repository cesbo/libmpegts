// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU
use std::io::{
    self,
    BufRead,
    Read
};

use bitwrap::{
    BitWrap,
    BitWrapError
};


mod tshead;
pub use tshead::*;

mod pcr;
pub use pcr::*;

#[cfg(test)]
mod tests;


pub const PID_NONE: u16 = 8192;
pub const PID_NULL: u16 = (PID_NONE - 1);
pub const PACKET_SIZE: usize = 188;


/// TS Null Packet.
/// Null packets are intended for padding of Transport Streams.
pub const NULL_PACKET: &[u8] = &[
    0x47, 0x1F, 0xFF, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
];


/// Hack for TS packet padding
pub const FILL_PACKET: &[u8] = &[
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
];


#[derive(Debug, Error)]
pub enum TSError {
    #[error_from]
    IO(io::Error),
    #[error_from]
    BitWrap(BitWrapError),
}


pub type Result<T> = std::result::Result<T, TSError>;


pub struct TS {
    head: TSHead,
    pub data: Vec<u8>,
    data_offset: usize,
}


impl TS {
    pub fn new() -> Self {
        Self {
            head: TSHead::default(),
            data: Vec::with_capacity(188),
            data_offset: 0,
        }
    }

    pub fn fill_packet() -> Self {
        Self {
            head: TSHead::default(),
            data: FILL_PACKET.to_vec(),
            data_offset: 0,
        }
    }

    pub fn set_data(&mut self, reader: &mut dyn Read) -> Result<()> {
        self.head.pack(&mut self.data[.. 3])?;
        reader.read_exact(&mut self.data[4 ..])?;
        Ok(())
    }

    /// Returns `true` if packet has valid sync byte.
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.head.is_sync()
    }

    /// Returns `true` if the transport error indicator is set
    #[inline]
    pub fn is_error(&self) -> bool {
        self.head.flag_error
    }

    /// Returns `true` if packet contains payload.
    #[inline]
    pub fn is_payload(&self) -> bool { 
        self.head.is_payload()
    }

    /// Returns `true` if payload begins in the packet.
    /// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
    /// Pointer field is a offset value, if `0` then payload starts immediately after it.
    #[inline]
    pub fn is_pusi(&self) -> bool {
        self.head.flag_payload_start
    }

    /// Returns `true` if packet contain adaptation field.
    /// Adaptation field locates after TS header.
    #[inline]
    pub fn is_adaptation(&self) -> bool {
        self.head.is_adaptation()
    }

    /// Returns payload offset in the TS packet
    /// Sum of the TS header size and adaptation field if exists.
    /// If TS packet without payload or offset value is invalid returns `0`
    /// In the PSI packets the `pointer field` is a part of payload, so it do not sums.
    #[inline]
    pub fn get_payload_offset(&self) -> u8 {
        if ! self.is_adaptation() {
            4
        } else {
            4 + 1 + self.get_adaptation_size()
        }
    }

    /// Returns `true` if the payload is scrambled.
    /// Actually this is only flag and packet contain could be not scrambled.
    #[inline]
    pub fn is_scrambled(&self) -> bool { 
        self.head.scrambled == 0x0C
    }

    /// Returns the size of the adaptation field.
    /// Function should be used if [`is_adaptation`] is `true`
    ///
    /// [`is_adaptation`]: #method.is_adaptation
    #[inline]
    pub fn get_adaptation_size(&self) -> u8 {
        self.data[4]
    }


    /// Returns PID - TS Packet identifier
    #[inline]
    pub fn get_pid(&self) -> u16 { 
        self.head.pid
    }


    /// Returns CC - TS Packet Continuity Counter
    /// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
    #[inline]
    pub fn get_cc(&self) -> u8 {
        self.head.cc
    }


    /// Sets PID
    #[inline]
    pub fn set_pid(&mut self, pid: u16) {
        debug_assert!(pid < 8192);
        self.head.pid = pid;
    }

    #[inline]
    pub fn set_cc(&mut self, cc: u8) {
        debug_assert!(cc < 16);
        self.head.cc = cc;
    }

    #[inline]
    pub fn set_payload_0(&mut self) {
        self.head.adaptation_field_control &= !0x01
    }

    #[inline]
    pub fn set_payload_1(&mut self) {
        self.head.adaptation_field_control |= 0x01
    }

    #[inline]
    pub fn set_pusi(&mut self, flag_payload_start: bool) {
        self.head.flag_payload_start = flag_payload_start;
    }

    /// === PCR functions ===
    /// 
    /// Returns `true` if TS packet has PCR field
    #[inline]
    pub fn is_pcr(&self) -> bool {
        self.is_adaptation() && self.get_adaptation_size() >= 7 && (self.data[5] & 0x10) != 0
    }


    /// Sets PCR value
    #[inline]
    pub fn set_pcr(&mut self, pcr: u64) {
        let pcr_base = pcr / 300;
        let pcr_ext = pcr % 300;

        self.data[6] = ((pcr_base >> 25) & 0xFF) as u8;
        self.data[7] = ((pcr_base >> 17) & 0xFF) as u8;
        self.data[8] = ((pcr_base >> 9) & 0xFF) as u8;
        self.data[9] = ((pcr_base >> 1) & 0xFF) as u8;
        self.data[10] = (((pcr_base << 7) & 0x80) as u8) | 0x7E | (((pcr_ext >> 8) & 0x01) as u8);
        self.data[11] = (pcr_ext & 0xFF) as u8;
    }


    /// Gets PCR value
    #[inline]
    pub fn get_pcr(&self) -> u64 {
        let pcr_base =
            (u64::from(self.data[6]) << 25) |
            (u64::from(self.data[7]) << 17) |
            (u64::from(self.data[8]) <<  9) |
            (u64::from(self.data[9]) <<  1) |
            (u64::from(self.data[10]) >>  7);

        let pcr_ext =
            (u64::from(self.data[10] & 0x01) << 8) | u64::from(self.data[11]);

        pcr_base * 300 + pcr_ext
    }
}


impl Read for TS {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut data = &self.data[self.data_offset ..];
        let read = data.read(buf)?;
        self.data_offset += read;
        Ok(read)
    }
}
