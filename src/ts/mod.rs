// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU
use std::io::{
    self,
    Read
};


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
#[error_prefix = "TS"]
pub enum TSError {
    #[error_from]
    IO(io::Error),
    #[error_kind("Empty data")]
    EmptyData,
}


pub type Result<T> = std::result::Result<T, TSError>;


#[derive(Default)]
pub struct TS<'a> {
    pub data: &'a mut [u8],
    data_offset: usize,
}


impl<'a> TS<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self {
            data,
            data_offset: 0,
        }
    }

    pub fn set_data<R: ?Sized>(&mut self, reader: &mut R) -> Result<()>
    where 
        R: Read
    {
        self.check_len(4)?;
        reader.read_exact(&mut self.data[4 .. 188])?;
        Ok(())
    }

    /// Returns `true` if packet has valid sync byte.
    #[inline]
    pub fn is_sync(&self) -> Result<bool> {
        self.check_len(1)?;
        Ok(self.data[0] == 0x47)
    }

    /// Returns `true` if the transport error indicator is set
    #[inline]
    pub fn is_error(&self) -> Result<bool> {
        self.check_len(2)?;
        Ok((self.data[1] & 0x80) != 0x00)
    }

    /// Returns `true` if packet contains payload.
    #[inline]
    pub fn is_payload(&self) -> Result<bool> {
        self.check_len(4)?;
        Ok((self.data[3] & 0x10) != 0x00)
    }

    /// Returns `true` if payload begins in the packet.
    /// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
    /// Pointer field is a offset value, if `0` then payload starts immediately after it.
    #[inline]
    pub fn is_pusi(&self) -> Result<bool> {
        self.check_len(2)?;
        Ok((self.data[1] & 0x40) != 0x00)
    }

    /// Returns `true` if packet contain adaptation field.
    /// Adaptation field locates after TS header.
    #[inline]
    pub fn is_adaptation(&self) -> Result<bool> {
        self.check_len(4)?;
        Ok((self.data[3] & 0x20) != 0x00)
    }

    /// Returns payload offset in the TS packet
    /// Sum of the TS header size and adaptation field if exists.
    /// If TS packet without payload or offset value is invalid returns `0`
    /// In the PSI packets the `pointer field` is a part of payload, so it do not sums.
    #[inline]
    pub fn get_payload_offset(&self) -> Result<u8> {
        if ! self.is_adaptation()? {
            Ok(4)
        } else {
            Ok(4 + 1 + self.get_adaptation_size()?)
        }
    }

    /// Returns `true` if the payload is scrambled.
    /// Actually this is only flag and packet contain could be not scrambled.
    #[inline]
    pub fn is_scrambled(&self) -> Result<bool> {
        self.check_len(4)?;
        Ok((self.data[3] & 0x20) != 0x00)
    }

    /// Returns the size of the adaptation field.
    /// Function should be used if [`is_adaptation`] is `true`
    ///
    /// [`is_adaptation`]: #method.is_adaptation
    #[inline]
    pub fn get_adaptation_size(&self) -> Result<u8> {
        self.check_len(5)?;
        Ok(self.data[4])
    }


    /// Returns PID - TS Packet identifier
    #[inline]
    pub fn get_pid(&self) -> Result<u16> {
        self.check_len(3)?;
        Ok((u16::from(self.data[1] & 0x1F) << 8) | u16::from(self.data[2]))
    }


    /// Returns CC - TS Packet Continuity Counter
    /// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
    #[inline]
    pub fn get_cc(&self) -> Result<u8> {
        self.check_len(4)?;
        Ok(self.data[3] & 0x0F)
    }


    /// Sets PID
    #[inline]
    pub fn set_pid(&mut self, pid: u16) -> Result<()> {
        debug_assert!(pid < 8192);
        self.check_len(3)?;
        self.data[1] = (self.data[1] & 0xE0) | ((pid >> 8) as u8);
        self.data[2] = pid as u8;
        Ok(())
    }

    #[inline]
    pub fn set_cc(&mut self, cc: u8) -> Result<()> {
        debug_assert!(cc < 16);
        self.check_len(4)?;
        self.data[3] = (self.data[3] & 0xF0) | (cc & 0x0F);
        Ok(())
    }

    
    #[inline]
    pub fn set_payload_0(&mut self) -> Result<()> {
        self.check_len(4)?;
        self.data[3] &= !0x10;
        Ok(())
    }


    #[inline]
    pub fn set_payload_1(&mut self) -> Result<()> {
        self.check_len(4)?;
        self.data[3] |= 0x10;
        Ok(())
    }


    #[inline]
    pub fn set_pusi_0(&mut self) -> Result<()> {
        self.check_len(2)?;
        self.data[1] &= !0x40;
        Ok(())
    }


    #[inline]
    pub fn set_pusi_1(&mut self) -> Result<()> {
        self.check_len(2)?;
        self.data[1] |= 0x40;
        Ok(())
    }


    /// === PCR functions ===
    /// 
    /// Returns `true` if TS packet has PCR field
    #[inline]
    pub fn is_pcr(&self) -> Result<bool> {
        self.check_len(6)?;
        Ok(self.is_adaptation()? && self.get_adaptation_size()? >= 7 && (self.data[5] & 0x10) != 0)
    }


    /// Sets PCR value
    #[inline]
    pub fn set_pcr(&mut self, pcr: u64) -> Result<()> {
        self.check_len(12)?;
        let pcr_base = pcr / 300;
        let pcr_ext = pcr % 300;

        self.data[6] = ((pcr_base >> 25) & 0xFF) as u8;
        self.data[7] = ((pcr_base >> 17) & 0xFF) as u8;
        self.data[8] = ((pcr_base >> 9) & 0xFF) as u8;
        self.data[9] = ((pcr_base >> 1) & 0xFF) as u8;
        self.data[10] = (((pcr_base << 7) & 0x80) as u8) | 0x7E | (((pcr_ext >> 8) & 0x01) as u8);
        self.data[11] = (pcr_ext & 0xFF) as u8;
        Ok(())
    }


    /// Gets PCR value
    #[inline]
    pub fn get_pcr(&self) -> Result<u64> {
        self.check_len(11)?;
        let pcr_base =
            (u64::from(self.data[6]) << 25) |
            (u64::from(self.data[7]) << 17) |
            (u64::from(self.data[8]) <<  9) |
            (u64::from(self.data[9]) <<  1) |
            (u64::from(self.data[10]) >>  7);

        let pcr_ext =
            (u64::from(self.data[10] & 0x01) << 8) | u64::from(self.data[11]);

        Ok(pcr_base * 300 + pcr_ext)
    }

    #[inline]
    fn check_len(&self, len: usize) -> Result<()> {
        if self.data.len() < len {
            Err(TSError::EmptyData)
        } else {
            Ok(())
        }
    }
}


impl Read for TS<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut data = &self.data[self.data_offset ..];
        let read = data.read(buf)?;
        self.data_offset += read;
        Ok(read)
    }
}
