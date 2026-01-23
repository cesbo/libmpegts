use crate::ts::{
    PACKET_SIZE,
    SYNC_BYTE,
    TsPacketRef,
};

/// Finds the position of a valid sync byte in data.
///
/// Returns the position of the first sync byte. If data is large enough,
/// validates by checking for a second sync byte at PACKET_SIZE offset.
#[inline]
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
/// Automatically handles sync byte detection and recovery from sync loss.
pub struct TsSlicer {
    buffer: [u8; PACKET_SIZE],
    fill: usize,
}

impl Default for TsSlicer {
    fn default() -> Self {
        Self {
            buffer: [0u8; PACKET_SIZE],
            fill: 0,
        }
    }
}

impl TsSlicer {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the internal buffer, discarding any partial packet.
    #[inline]
    pub fn reset(&mut self) {
        self.fill = 0;
    }

    /// Slices input data into TS packets.
    ///
    /// Performs sync byte validation:
    /// - Finds first valid sync byte in input data
    /// - Validates sync byte on completing a partial packet
    /// - Re-synchronizes on sync loss
    pub fn slice<'a>(&'a mut self, data: &'a [u8]) -> TsSlicerIter<'a> {
        if data.is_empty() {
            return TsSlicerIter::new(self, &[]);
        }

        if self.fill > 0 {
            // Have partial packet in buffer, try to complete it
            let remain = PACKET_SIZE - self.fill;

            if data.len() > remain {
                // Can verify sync byte after completing the packet
                if data[remain] != SYNC_BYTE {
                    // Sync lost, reset buffer and re-sync
                    self.fill = 0;
                } else {
                    // Sync OK, complete the partial packet
                    self.buffer[self.fill ..].copy_from_slice(&data[.. remain]);
                    self.fill = PACKET_SIZE;
                    return TsSlicerIter::new(self, &data[remain ..]);
                }
            } else {
                let end = self.fill + data.len();
                self.buffer[self.fill .. end].copy_from_slice(data);
                self.fill = end;
                return TsSlicerIter::new(self, &[]);
            }
        }

        if let Some(skip) = find_sync(data) {
            TsSlicerIter::new(self, &data[skip ..])
        } else {
            // No sync found, discard all data
            TsSlicerIter::new(self, &[])
        }
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
    #[inline]
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
                let slice = self.slicer.buffer.as_ptr() as *const [u8; PACKET_SIZE];
                return Some(TsPacketRef::from(unsafe { &*slice }));
            } else {
                return None;
            }
        }

        let remain = self.data.len() - self.skip;

        if remain >= PACKET_SIZE {
            let end = self.skip + PACKET_SIZE;
            let packet = &self.data[self.skip .. end];
            self.skip = end;
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
