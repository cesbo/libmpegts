use crate::ts::{
    PACKET_SIZE,
    SYNC_BYTE,
    TsPacketRef,
};

/// Finds the position of a valid sync byte in data.
///
/// Returns the position of the first sync byte. If data is large enough,
/// validates by checking for a second sync byte at PACKET_SIZE offset.
fn find_sync(data: &[u8]) -> Option<usize> {
    let mut pos = 0;

    while data.len() > pos {
        if data[pos] == SYNC_BYTE {
            let next = pos + PACKET_SIZE;
            if data.len() > next {
                if data[next] == SYNC_BYTE {
                    return Some(pos);
                }
                // First sync was false positive, continue searching
            } else {
                // Not enough data to verify, trust this sync byte
                return Some(pos);
            }
        }
        pos += 1;
    }

    None
}

/// Stateful slicer for MPEG-TS packets.
///
/// Buffers partial packets across multiple input chunks, enabling processing
/// of arbitrary-length byte slices. Uses zero-copy for packets within input slice,
/// copying only for the single packet that may span two input chunks.
///
/// Checks the sync byte on every packet boundary and re-synchronizes on loss.
/// Sync is held once a packet has been delivered. Counters are cumulative
/// and update as the iterator advances:
/// - [`sync_losses`](Self::sync_losses) - packet boundaries without a sync byte while sync was held
/// - [`skipped_bytes`](Self::skipped_bytes) - input bytes dropped while searching for sync
pub struct TsSlicer {
    buffer: [u8; PACKET_SIZE],
    fill: usize,
    synced: bool,
    sync_losses: u64,
    skipped_bytes: u64,
}

impl Default for TsSlicer {
    fn default() -> Self {
        Self {
            buffer: [0u8; PACKET_SIZE],
            fill: 0,
            synced: false,
            sync_losses: 0,
            skipped_bytes: 0,
        }
    }
}

impl TsSlicer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new stream: discards any partial packet and drops sync,
    /// so the next data is searched for a sync byte as the first chunk is.
    ///
    /// The counters survive: the discarded partial packet is not counted
    /// as skipped, and the next sync search counts no loss.
    pub fn reset(&mut self) {
        self.fill = 0;
        self.synced = false;
    }

    /// Number of packet boundaries without a sync byte while sync was held.
    ///
    /// A run of garbage counts once, however many chunks it spans.
    pub fn sync_losses(&self) -> u64 {
        self.sync_losses
    }

    /// Number of input bytes dropped while searching for sync, including
    /// a sync candidate buffered at the end of a chunk and discarded because
    /// the next chunk has no sync byte where its packet ends.
    pub fn skipped_bytes(&self) -> u64 {
        self.skipped_bytes
    }

    /// Slices input data into TS packets.
    ///
    /// Performs sync byte validation:
    /// - Finds first valid sync byte in input data
    /// - Confirms a sync candidate buffered at the end of a chunk by the sync byte after its packet
    /// - Validates sync byte on every packet boundary and re-synchronizes on loss
    pub fn slice<'a>(&'a mut self, data: &'a [u8]) -> TsSlicerIter<'a> {
        if self.fill > 0 {
            // Have partial packet in buffer, try to complete it
            let remain = PACKET_SIZE - self.fill;

            if data.len() <= remain {
                let end = self.fill + data.len();
                self.buffer[self.fill .. end].copy_from_slice(data);
                self.fill = end;
                return TsSlicerIter::new(self, &[]);
            }

            // With sync held, the boundary after the packet is checked by the
            // iterator, as inside a chunk
            if self.synced || data[remain] == SYNC_BYTE {
                self.buffer[self.fill ..].copy_from_slice(&data[.. remain]);
                self.fill = PACKET_SIZE;
                return TsSlicerIter::new(self, &data[remain ..]);
            }

            // Unconfirmed sync candidate: drop it and re-sync
            self.skipped_bytes += self.fill as u64;
            self.fill = 0;
        }

        TsSlicerIter::new(self, data)
    }
}

/// Iterator over TS packets from a `TsSlicer`.
pub struct TsSlicerIter<'a> {
    slicer: &'a mut TsSlicer,
    data: &'a [u8],
    skip: usize,
}

impl<'a> TsSlicerIter<'a> {
    /// Returns remaining unprocessed data.
    fn new(slicer: &'a mut TsSlicer, data: &'a [u8]) -> Self {
        Self {
            slicer,
            data,
            skip: 0,
        }
    }
}

impl<'a> Iterator for TsSlicerIter<'a> {
    type Item = TsPacketRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slicer.fill > 0 {
            if self.slicer.fill == PACKET_SIZE {
                self.slicer.fill = 0;
                self.slicer.synced = true;
                let slice = self.slicer.buffer.as_ptr() as *const [u8; PACKET_SIZE];
                return Some(TsPacketRef::from(unsafe { &*slice }));
            } else {
                return None;
            }
        }

        if self.skip == self.data.len() {
            return None;
        }

        if !self.slicer.synced || self.data[self.skip] != SYNC_BYTE {
            if self.slicer.synced {
                self.slicer.synced = false;
                self.slicer.sync_losses += 1;
            }
            let rest = &self.data[self.skip ..];
            let skip = find_sync(rest).unwrap_or(rest.len());
            self.slicer.skipped_bytes += skip as u64;
            self.skip += skip;
        }

        let remain = self.data.len() - self.skip;

        if remain >= PACKET_SIZE {
            let end = self.skip + PACKET_SIZE;
            let packet = &self.data[self.skip .. end];
            self.skip = end;
            self.slicer.synced = true;
            let slice = packet.as_ptr() as *const [u8; PACKET_SIZE];
            return Some(TsPacketRef::from(unsafe { &*slice }));
        }

        if remain > 0 {
            // Buffer the remaining partial packet
            self.slicer.buffer[.. remain].copy_from_slice(&self.data[self.skip ..]);
            self.slicer.fill = remain;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NULL packet with 0xFF payload: no stray sync byte on any stride
    fn packet(cc: u8) -> [u8; PACKET_SIZE] {
        let mut packet = [0xFF; PACKET_SIZE];
        packet[.. 4].copy_from_slice(&[SYNC_BYTE, 0x1F, 0xFF, 0x10 | cc]);
        packet
    }

    fn feed(slicer: &mut TsSlicer, chunks: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in chunks {
            out.extend(slicer.slice(chunk).map(|p| p.cc()));
        }
        out
    }

    fn counts(slicer: &TsSlicer) -> (u64, u64) {
        (slicer.sync_losses(), slicer.skipped_bytes())
    }

    // slices the stream whole and in chunks of every size; all must agree
    fn feed_any_chunking(ts: &[u8]) -> (Vec<u8>, (u64, u64)) {
        let mut slicer = TsSlicer::new();
        let whole = (feed(&mut slicer, &[ts]), counts(&slicer));

        for size in 1 .. ts.len() {
            let chunks: Vec<&[u8]> = ts.chunks(size).collect();
            let mut slicer = TsSlicer::new();
            let ccs = feed(&mut slicer, &chunks);
            assert_eq!((ccs, counts(&slicer)), whole, "chunk size {size}");
        }

        whole
    }

    #[test]
    fn passes_clean_stream() {
        let ts = [packet(0), packet(1), packet(2), packet(3)].concat();

        assert_eq!(feed_any_chunking(&ts), (vec![0, 1, 2, 3], (0, 0)));
    }

    #[test]
    fn skips_leading_garbage() {
        // the stray sync byte is not confirmed 188 bytes on
        let lead = [SYNC_BYTE, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let ts = [&lead[..], &packet(0), &packet(1)].concat();
        let mut slicer = TsSlicer::new();

        assert_eq!(feed(&mut slicer, &[&ts]), [0, 1]);
        assert_eq!(counts(&slicer), (0, 7));
    }

    #[test]
    fn counts_slip() {
        let ts = [
            &packet(0)[..],
            &packet(1),
            &packet(2),
            &[0x00; 5],
            &packet(3),
            &packet(4),
            &packet(5),
        ]
        .concat();

        assert_eq!(feed_any_chunking(&ts), (vec![0, 1, 2, 3, 4, 5], (1, 5)));
    }

    #[test]
    fn counts_short_packet() {
        // packet 1 is cut to 150 bytes: it is delivered with the head of
        // packet 2, the boundary after it has no sync byte, the tail of
        // packet 2 is skipped
        let ts = [&packet(0)[..], &packet(1)[.. 150], &packet(2), &packet(3)].concat();

        assert_eq!(feed_any_chunking(&ts), (vec![0, 1, 3], (1, 150)));
    }

    #[test]
    fn counts_garbage_across_chunks() {
        // leading garbage: the sync byte at the end of the first chunk cannot
        // be confirmed, is buffered, then dropped without a loss
        let lead = [&[0x00; 10][..], &[SYNC_BYTE, 0x00, 0x00], &[0x00; 4]].concat();
        // garbage after sync is held: one loss for the whole run, and a
        // chunk ending in more than a packet of it delivers nothing
        let ts = [
            &lead[..],
            &packet(0),
            &packet(1),
            &[0x00; 200],
            &packet(2),
            &packet(3),
        ]
        .concat();
        let mut slicer = TsSlicer::new();

        let split = 17 + 2 * PACKET_SIZE + 190;
        let ccs = feed(&mut slicer, &[&ts[.. 13], &ts[13 .. split], &ts[split ..]]);

        assert_eq!(ccs, [0, 1, 2, 3]);
        assert_eq!(counts(&slicer), (1, 217));
    }

    #[test]
    fn keeps_counters_on_reset() {
        let head = [&[0x00; 3][..], &packet(0), &packet(1)[.. 50]].concat();
        let ts = [&[0x00; 3][..], &packet(2), &packet(3)].concat();
        let mut slicer = TsSlicer::new();

        assert_eq!(feed(&mut slicer, &[&head]), [0]);
        assert_eq!(counts(&slicer), (0, 3));
        slicer.reset();
        // a new stream: the dropped partial is not counted, garbage before
        // the first packet is skipped without a loss
        assert_eq!(feed(&mut slicer, &[&ts]), [2, 3]);
        assert_eq!(counts(&slicer), (0, 6));
    }
}
