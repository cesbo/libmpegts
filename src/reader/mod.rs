use std::{
    cmp,
    fmt,
    io::{
        self,
        Read,
        BufRead,
    },
};

use crate::{
    psi::{
        Psi,
        PsiDemux,
        Pat,
    },
    ts,
};


pub trait TsRead: fmt::Debug + Read {
    // TODO: fn for stream info (service iterator)
}


const DEFAULT_BUF_SIZE: usize = 7 * ts::PACKET_SIZE;


struct PsiItem {
    pid: u16,
    tid: u16, /* tsid for PAT, service_id for PMT, ... */
    psi: Psi,
}


pub struct TsReader<R> {
    inner: R,

    buf: Box<[u8]>,
    pos: usize,     // reading position
    cap: usize,     // last byte
    rem: usize,     // remain bytes after cap (cap aligned to ts::PACKET_SIZE)
}


impl<R: fmt::Debug + Read> TsReader<R> {
    pub fn new(inner: R) -> TsReader<R> {
        TsReader {
            inner,

            buf:{
                let mut v = Vec::with_capacity(DEFAULT_BUF_SIZE);
                unsafe { v.set_len(DEFAULT_BUF_SIZE) };
                v.into_boxed_slice()
            },
            pos: 0,
            cap: 0,
            rem: 0,
        }
    }
}


impl<R> TsReader<R> {
    /// Gets a reference to the underlying reader.
    #[inline]
    pub fn get_ref(&self) -> &R { &self.inner }

    /// Gets a mutable reference to the underlying reader.
    #[inline]
    pub fn get_mut(&mut self) -> &mut R { &mut self.inner }

    /// Unwraps this `TsReader`, returning the underlying reader.
    #[inline]
    pub fn into_inner(self) -> R { self.inner }

    /// Parses TS packets
    fn parse(&mut self, range: std::ops::Range<usize>) {
        let packet = &self.buf[range];
        let _pid = ts::get_pid(packet);

        // TODO: parse
    }
}


impl<R: fmt::Debug> fmt::Debug for TsReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsReader")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("cap", &self.cap)
            .field("rem", &self.rem)
            .finish()
    }
}


impl<R: Read> Read for TsReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut rem = self.fill_buf()?;
        if ! rem.is_empty() {
            let nread = rem.read(buf)?;
            self.consume(nread);
            Ok(nread)
        } else {
            Ok(0)
        }
    }
}


impl<R: Read> BufRead for TsReader<R> {
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos < self.cap {
            return Ok(&self.buf[self.pos .. self.cap])
        }

        if self.rem != 0 {
            unsafe {
                std::ptr::copy(
                    self.buf[self.cap ..].as_ptr() as *const u8,
                    self.buf.as_mut_ptr() as *mut u8,
                    self.rem)
            };
            self.cap = self.rem;
            self.rem = 0;
        } else {
            self.cap = 0;
        }

        self.pos = 0;

        while self.pos + ts::PACKET_SIZE > self.cap {
            let x = self.inner.read(&mut self.buf[self.cap ..])?;
            if x == 0 {
                return Ok(&[])
            }
            self.cap += x;
        }

        // TODO: seek sync byte

        let mut skip = self.pos;
        while skip < self.cap {
            let next = skip + ts::PACKET_SIZE;
            self.parse(skip .. next);
            skip = next;
        }

        Ok(&self.buf[self.pos .. self.cap])
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.cap, self.pos + amt);
    }
}


impl<R: fmt::Debug + Read> TsRead for TsReader<R> {

}
