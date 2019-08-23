// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU


/// Bytes operations with array
pub trait Bytes {
    fn get_u16(&self) -> u16;
    fn set_u16(&mut self, value: u16);
    fn get_u24(&self) -> u32;
    fn set_u24(&mut self, value: u32);
    fn get_u32(&self) -> u32;
    fn set_u32(&mut self, value: u32);
}


impl Bytes for [u8] {
    #[inline]
    fn get_u16(&self) -> u16 {
        debug_assert!(self.len() >= 2);
        (u16::from(self[0]) << 8) | u16::from(self[1])
    }

    #[inline]
    fn set_u16(&mut self, value: u16) {
        debug_assert!(self.len() >= 2);
        self[0] = (value >> 8) as u8;
        self[1] = (value) as u8;
    }

    #[inline]
    fn get_u24(&self) -> u32 {
        debug_assert!(self.len() >= 3);
        (u32::from(self[0]) << 16) | (u32::from(self[1]) << 8) | u32::from(self[2])
    }

    #[inline]
    fn set_u24(&mut self, value: u32) {
        debug_assert!(self.len() >= 3);
        self[0] = (value >> 16) as u8;
        self[1] = (value >> 8) as u8;
        self[2] = (value) as u8;
    }

    #[inline]
    fn get_u32(&self) -> u32 {
        debug_assert!(self.len() >= 4);
        (u32::from(self[0]) << 24) | (u32::from(self[1]) << 16) | (u32::from(self[2]) << 8) | u32::from(self[3])
    }

    #[inline]
    fn set_u32(&mut self, value: u32) {
        debug_assert!(self.len() >= 4);
        self[0] = (value >> 24) as u8;
        self[1] = (value >> 16) as u8;
        self[2] = (value >> 8) as u8;
        self[3] = (value) as u8;
    }
}


#[cfg(test)]
mod tests {
    use crate::bytes::*;

    #[test]
    fn test_get_bytes_u16() {
        let data: &[u8] = &[0x12, 0x34];
        assert_eq!(data[0 ..].get_u16(), 0x1234);
    }

    #[test]
    fn test_set_bytes_u16() {
        let mut data = Vec::<u8>::new();
        data.resize(2, 0x00);
        data[0 ..].set_u16(0x1234);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
    }

    #[test]
    fn test_get_bytes_u24() {
        let data: &[u8] = &[0x12, 0x34, 0xAB];
        assert_eq!(data[0 ..].get_u24(), 0x1234AB);
    }

    #[test]
    fn test_set_bytes_u24() {
        let mut data = Vec::<u8>::new();
        data.resize(3, 0x00);
        data[0 ..].set_u24(0x1234AB);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0xAB);
    }

    #[test]
    fn test_get_bytes_u32() {
        let data: &[u8] = &[0x12, 0x34, 0xAB, 0xCD];
        assert_eq!(data[0 ..].get_u32(), 0x1234ABCD);
    }

    #[test]
    fn test_set_bytes_u32() {
        let mut data = Vec::<u8>::new();
        data.resize(4, 0x00);
        data[0 ..].set_u32(0x1234ABCD);
        assert_eq!(data[0], 0x12);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0xAB);
        assert_eq!(data[3], 0xCD);
    }
}
