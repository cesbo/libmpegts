use std::{
    cmp,
    fmt,
    io::{
        self,
        Read,
        BufRead,
    },
};


pub trait TsRead: fmt::Debug + Read {
    // TODO: fn for stream info (service iterator)
}


const DEFAULT_BUF_SIZE: usize = 8 * 1024 / 188 * 188;


pub struct TsReader<R> {
    inner: R,

    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
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
}


impl<R: fmt::Debug> fmt::Debug for TsReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsReader")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("cap", &self.cap)
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
        if self.pos >= self.cap {
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;

            // TODO: parse
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
