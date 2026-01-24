//! PES Packetizer - converts PES packets into TS packets

use bytes::BytesMut;

use super::PesHeader;
use crate::ts::{
    PACKET_SIZE,
    TsPacketMut,
};

/// Queue growth step: 2MB
const QUEUE_GROWTH_STEP: usize = 2 * 1024 * 1024;

/// TS packet payload capacity (without adaptation field)
const TS_PAYLOAD_SIZE: usize = PACKET_SIZE - 4; // 184 bytes

/// PES Packetizer - splits PES data into TS packets
///
/// # Example
/// ```ignore
/// use mpegts::pes::{PesHeader, PesPacketizer, STREAM_ID_VIDEO};
///
/// let mut packetizer = PesPacketizer::new(0x100); // video PID
///
/// let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(90000);
/// let es_data = vec![0u8; 1000]; // video frame
///
/// packetizer.packetize(&header, &es_data);
///
/// while let Some(ts_packet) = packetizer.pop() {
///     // send ts_packet to muxer
/// }
/// ```
pub struct PesPacketizer {
    /// Packet Identifier for TS packets
    pid: u16,
    /// Continuity counter (0-15, wraps around)
    cc: u8,
    /// Queue of ready TS packets
    queue: BytesMut,
}

impl PesPacketizer {
    /// Creates new PesPacketizer with given PID
    pub fn new(pid: u16) -> Self {
        Self {
            pid,
            cc: 0,
            queue: BytesMut::with_capacity(QUEUE_GROWTH_STEP),
        }
    }

    /// Returns number of TS packets in queue
    pub fn len(&self) -> usize {
        self.queue.len() / PACKET_SIZE
    }

    /// Returns true if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Packetizes PES header + ES payload into TS packets
    ///
    /// First TS packet will have PUSI=1, subsequent packets PUSI=0.
    /// Last packet will be padded with adaptation field stuffing if needed.
    pub fn packetize(&mut self, header: &PesHeader, payload: &[u8]) {
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

        // Ensure queue has capacity
        let needed_bytes = total_packets * PACKET_SIZE;
        if needed_bytes > self.queue.capacity() - self.queue.len() {
            self.queue.reserve(QUEUE_GROWTH_STEP.max(needed_bytes));
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

            self.queue.extend_from_slice(&packet_buf);
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

            // Add packet to queue
            self.queue.extend_from_slice(&packet_buf);

            // Increment CC
            self.cc = (self.cc + 1) & 0x0F;
        }
    }

    /// Pops one TS packet from queue
    pub fn pop(&mut self) -> Option<[u8; PACKET_SIZE]> {
        if self.queue.len() < PACKET_SIZE {
            return None;
        }

        let bytes = self.queue.split_to(PACKET_SIZE);
        let mut packet = [0u8; PACKET_SIZE];
        packet.copy_from_slice(&bytes);
        Some(packet)
    }

    /// Clears the queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Resets continuity counter
    pub fn reset_cc(&mut self) {
        self.cc = 0;
    }

    // TODO: drain() -> impl Iterator<Item = [u8; PACKET_SIZE]>
    // TODO: pop_into(&mut [u8; PACKET_SIZE]) -> bool для zero-copy
}
