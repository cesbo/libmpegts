use crate::ts::{
    PACKET_SIZE,
    TsPacketRef,
};

/// Stateful slicer for MPEG-TS packets.
///
/// Buffers partial packets across multiple input chunks, enabling processing
/// of arbitrary-length byte slices. Uses zero-copy for packets within input slice,
/// copying only for the single packet that may span two input chunks.
///
/// # Example
///
/// ```
/// use mpegts::ts::TsSlicer;
///
/// let mut slicer = TsSlicer::new();
///
/// // Process chunks of arbitrary size
/// for chunk in data_source {
///     for packet in slicer.slice(&chunk) {
///         // Process packet
///     }
/// }
/// ```
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
    pub fn slice<'a>(&'a mut self, data: &'a [u8]) -> TsSlicerIter<'a> {
        let mut skip = 0;

        if self.fill > 0 {
            // Complete the partial packet in the buffer
            skip = data.len().min(PACKET_SIZE - self.fill);
            let end = self.fill + skip;
            self.buffer[self.fill .. end].copy_from_slice(&data[.. skip]);
            self.fill = end;
        }

        TsSlicerIter::new(self, &data[skip ..])
    }
}

/// Iterator over TS packets from a `TsSlicer`.
pub struct TsSlicerIter<'a> {
    slicer: &'a mut TsSlicer,
    data: &'a [u8],
    pos: usize,
}

impl<'a> TsSlicerIter<'a> {
    /// Returns remaining unprocessed data.
    #[inline]
    fn new(slicer: &'a mut TsSlicer, data: &'a [u8]) -> Self {
        Self {
            slicer,
            data,
            pos: 0,
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

        let remain = self.data.len() - self.pos;

        if remain >= PACKET_SIZE {
            let end = self.pos + PACKET_SIZE;
            let packet = &self.data[self.pos .. end];
            self.pos = end;
            let slice = packet.as_ptr() as *const [u8; PACKET_SIZE];
            return Some(TsPacketRef::from(unsafe { &*slice }));
        }

        if remain > 0 {
            // Buffer the remaining partial packet
            self.slicer.buffer[.. remain].copy_from_slice(&self.data[self.pos ..]);
            self.slicer.fill = remain;
        }

        None
    }
}
