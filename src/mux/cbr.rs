use std::{
    fmt,
    io,
};

use crate::{
    ts,
    reader::TsRead,
};


// max 500ms for 80Mbit/s
const DEFAULT_BUF_SIZE: usize = (80 * 1000 * 1000 / 8) * 500 / 1000;


struct Block {
    buffer: Box<[u8]>,
    /// Reading position
    pos: usize,
    /// Bytes in the buffer
    cap: usize,

    /// Last PCR value (packet with PCR in the next block)
    pcr: u64,
    /// Difference between first PCR (current block) and last PCR
    delta: u64,
}


impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HttpBuffer")
            .field("pos", &self.pos)
            .field("cap", &self.cap)
            .finish()
    }
}


impl Block {
    fn new() -> Self {
        Block {
            buffer: {
                let mut v = Vec::with_capacity(DEFAULT_BUF_SIZE);
                unsafe { v.set_len(DEFAULT_BUF_SIZE) };
                v.into_boxed_slice()
            },
            pos: 0,
            cap: 0,

            pcr: 0,
            delta: 0,
        }
    }
}


#[derive(Debug)]
pub struct Cbr<R> {
    inner: R,

    parity: usize,
    blocks: [Block; 2],

    pid: u16,
    last_pcr: u64,
}


impl<R: TsRead> Cbr<R> {
    pub fn new(inner: R) -> Self {
        Cbr {
            inner,

            parity: 0,
            blocks: [Block::new(), Block::new()],

            pid: ts::PID_NONE,
            last_pcr: ts::PCR_NONE,
        }
    }

    fn analyze(&mut self) {
        let block = &mut self.blocks[self.parity];
        let packet = &block.buffer[block.cap ..];

        if ! ts::is_pcr(packet) {
            return;
        }

        let pid = ts::get_pid(packet);
        if pid != self.pid {
            if self.pid != ts::PID_NONE {
                return;
            } else {
                self.pid = pid;
            }
        }

        let pcr = ts::get_pcr(packet);

        if self.last_pcr != ts::PCR_NONE {
            if self.last_pcr == pcr {
                return
            }

            // TODO: move packet to the next block, change parity
            // TODO: check pcr drift
            // TODO: set the buffer filled flag
            // TODO: calculate CBR
        }

        self.last_pcr = pcr;
    }

    fn push_packet(&mut self) -> io::Result<usize> {
        let block = &mut self.blocks[self.parity];
        // TODO: check cap overflow

        let packet = &mut block.buffer[block.cap ..];
        let x = self.inner.read(packet)?;
        if x == ts::PACKET_SIZE {
            self.analyze();

            let block = &mut self.blocks[self.parity];
            block.cap += ts::PACKET_SIZE;
        }
        Ok(x)
    }
}


impl<R: TsRead> TsRead for Cbr<R> {
    fn read(&mut self, packet: &mut [u8]) -> io::Result<usize> {
        // 1. is last buffer ready. if not then complete current buffer
        // TODO:

        // 2. send packet from last buffer or NULL TS
        // TODO:

        // 3. read packet to the current buffer if not filled
        // TODO:
        self.push_packet()
    }
}
