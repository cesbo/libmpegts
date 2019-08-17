// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use std::{
    fmt,
    io::{
        self,
        Read,
    },
};

use crate::{
    ts,
    reader::TsRead,
};


pub struct TsDrain<R> {
    inner: R,

    buf: Box<[u8]>,
    pos: usize,
}


impl<R: TsRead> fmt::Debug for TsDrain<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsDrain")
            .field("inner", &self.inner)
            .finish()
    }
}


impl<R: TsRead> TsDrain<R> {
    pub fn new(inner: R) -> Self {
        TsDrain {
            inner,

            buf: unsafe {
                let mut v = Vec::with_capacity(ts::PACKET_SIZE);
                v.set_len(ts::PACKET_SIZE);
                v.into_boxed_slice()
            },
            pos: 0,
        }
    }
}


impl<R: TsRead> Read for TsDrain<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos == 0 {
            if buf.len() >= ts::PACKET_SIZE {
                return self.inner.read(buf);
            }

            let x = self.inner.read(&mut self.buf)?;
            if x == 0 {
                return Ok(0);
            }
        }

        let x = self.buf[self.pos ..].as_ref().read(buf)?;
        self.pos = (self.pos + x) % ts::PACKET_SIZE;
        Ok(x)
    }
}
