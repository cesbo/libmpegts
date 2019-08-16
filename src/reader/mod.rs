use std::{
    fmt,
    io::{
        self,
        Read,
    },
};

use crate::ts;

mod drain;
pub use drain::TsDrain;


pub trait TsRead: fmt::Debug {
    fn read(&mut self, packet: &mut [u8]) -> io::Result<usize>;
    // TODO: fn for stream info (service iterator)
}


pub struct TsReader<R> {
    inner: R,
}


impl<R: fmt::Debug + Read> TsReader<R> {
    pub fn new(inner: R) -> TsReader<R> {
        TsReader {
            inner,
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
    fn parse(&mut self, packet: &[u8]) {
        let _pid = ts::get_pid(packet);

        // TODO: parse
    }
}


impl<R: fmt::Debug> fmt::Debug for TsReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsReader")
            .field("inner", &self.inner)
            .finish()
    }
}


impl<R: fmt::Debug + Read> TsRead for TsReader<R> {
    fn read(&mut self, packet: &mut [u8]) -> io::Result<usize> {
        assert!(packet.len() >= ts::PACKET_SIZE);

        let mut skip = 0;

        while skip == 0 {
            let x = self.inner.read(&mut packet[.. 1])?;
            if x == 0 {
                return Ok(0)
            }
            if ts::is_sync(packet) {
                skip = 1;
            }
        }

        while skip != ts::PACKET_SIZE {
            let x = self.inner.read(&mut packet[skip .. ts::PACKET_SIZE])?;
            if x == 0 {
                return Ok(0)
            }
            skip += x;
        }

        self.parse(packet);

        Ok(ts::PACKET_SIZE)
    }
}
