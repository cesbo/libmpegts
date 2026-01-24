//! PES Packetizer - converts PES packets into TS packets

use ringbuf::{
    HeapRb,
    traits::{
        Consumer,
        Observer,
        Producer,
    },
};

use super::{
    PesHeader,
    PesPacketizerError,
};
use crate::ts::{
    PACKET_SIZE,
    TsPacketMut,
};

/// TS packet payload capacity (without adaptation field)
const TS_PAYLOAD_SIZE: usize = PACKET_SIZE - 4;

/// PES Packetizer - splits PES data into TS packets
///
/// Uses a ring buffer for zero-copy reads. Data is written via `packetize()`
/// and read via `peek()`/`consume()` pattern.
///
/// # Example
/// ```ignore
/// use mpegts::pes::{PesHeader, PesPacketizer, STREAM_ID_VIDEO};
///
/// let mut packetizer = PesPacketizer::new(101, 1024 * 1024); // video PID, 1MB buffer
///
/// let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(90000);
/// let es_data = vec![0u8; 1000]; // video frame
///
/// packetizer.packetize(&header, &es_data).unwrap();
///
/// // Read data using peek/consume pattern
/// while packetizer.len() > 0 {
///     let data = packetizer.peek();
///     // process data (e.g., send to network)
///     let consumed = data.len();
///     packetizer.consume(consumed);
/// }
/// ```
pub struct PesPacketizer {
    pid: u16,
    cc: u8,
    rb: HeapRb<u8>,
}

impl PesPacketizer {
    /// Creates new PesPacketizer with given PID and buffer capacity in bytes
    pub fn new(pid: u16, capacity: usize) -> Self {
        Self {
            pid,
            cc: 0,
            rb: HeapRb::new(capacity),
        }
    }

    /// Returns number of bytes in buffer
    pub fn len(&self) -> usize {
        self.rb.occupied_len()
    }

    /// Returns true if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.rb.is_empty()
    }

    /// Returns available space in buffer
    pub fn available(&self) -> usize {
        self.rb.vacant_len()
    }

    /// Returns first contiguous slice of data in buffer
    ///
    /// Due to ring buffer wrap-around, this may not return all available data.
    /// The returned slice may contain partial TS packets.
    pub fn peek(&self) -> &[u8] {
        let (head, tail) = self.rb.as_slices();
        if !head.is_empty() { head } else { tail }
    }

    /// Consumes (removes) specified number of bytes from the buffer
    pub fn consume(&mut self, count: usize) {
        self.rb.skip(count);
    }

    /// Packetizes PES header + ES payload into TS packets
    pub fn packetize(
        &mut self,
        header: &PesHeader,
        payload: &[u8],
    ) -> Result<(), PesPacketizerError> {
        // Build PES header
        let mut pes_header_buf = [0u8; 32]; // Max PES header size
        let pes_header = {
            let len = header.write(&mut pes_header_buf);
            &pes_header_buf[.. len]
        };

        // Total data to packetize
        let total_len = pes_header.len() + payload.len();

        // Calculate how many TS packets we need
        let first_payload_capacity = TS_PAYLOAD_SIZE;
        let remaining_after_first = total_len.saturating_sub(first_payload_capacity);
        let additional_packets = remaining_after_first.div_ceil(TS_PAYLOAD_SIZE);
        let total_packets = 1 + additional_packets;

        // Check buffer capacity
        let needed_bytes = total_packets * PACKET_SIZE;
        let available = self.rb.vacant_len();
        if needed_bytes > available {
            return Err(PesPacketizerError::BufferFull {
                required: needed_bytes,
                available,
            });
        }

        // TS packet buffer
        let mut packet_buf = [0u8; PACKET_SIZE];
        let mut payload_offset;

        // First packet with PES header
        {
            let mut packet = TsPacketMut::from(&mut packet_buf);
            packet.set_sync();
            packet.set_pid(self.pid);
            packet.set_payload();

            let (payload_size, stuffing) = if total_len > TS_PAYLOAD_SIZE {
                (TS_PAYLOAD_SIZE, 0)
            } else {
                (total_len, TS_PAYLOAD_SIZE - total_len)
            };

            packet.set_cc(self.cc);
            if stuffing > 0 {
                packet.write_stuffing(stuffing);
            }
            packet.set_pusi();

            let packet_offset = 4 + stuffing;
            let end = packet_offset + pes_header.len();
            packet_buf[packet_offset .. end].copy_from_slice(pes_header);

            let payload_end = payload_size - pes_header.len();
            packet_buf[end ..].copy_from_slice(&payload[.. payload_end]);
            payload_offset = payload_end;

            self.rb.push_slice(&packet_buf);
            self.cc = (self.cc + 1) & 0x0F;
        }

        // Subsequent packets with ES payload only
        while payload_offset < payload.len() {
            // Determine payload size for this packet and stuffing if needed
            let remain = payload.len() - payload_offset;
            let (payload_size, stuffing) = if remain > TS_PAYLOAD_SIZE {
                (TS_PAYLOAD_SIZE, 0)
            } else {
                (remain, TS_PAYLOAD_SIZE - remain)
            };

            // Build TS packet
            let mut packet = TsPacketMut::from(&mut packet_buf);
            packet.set_cc(self.cc);
            if stuffing > 0 {
                packet.write_stuffing(stuffing);
            }
            packet.clear_pusi();

            let packet_offset = 4 + stuffing;
            let payload_end = payload_offset + payload_size;
            packet_buf[packet_offset ..].copy_from_slice(&payload[payload_offset .. payload_end]);
            payload_offset = payload_end;

            // Add packet to ring buffer
            self.rb.push_slice(&packet_buf);

            // Increment CC
            self.cc = (self.cc + 1) & 0x0F;
        }

        Ok(())
    }

    /// Clears the buffer
    pub fn clear(&mut self) {
        self.rb.clear();
    }

    /// Resets continuity counter
    pub fn reset_cc(&mut self) {
        self.cc = 0;
    }
}
