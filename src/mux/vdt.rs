use std::cmp::Ordering;

/// Virtual Delivery Time for packet scheduling.
///
/// Advances once per emitted packet. Fractional frame duration is spread
/// across packets with a Bresenham-style accumulator.
#[derive(Default)]
pub struct Vdt {
    value: u64,
    frame_packets: u64,
    step: u64,
    remainder: u64,
    error: u64,
}

impl Vdt {
    pub fn set_value(&mut self, value: u64) {
        self.value = value;
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    /// Prepares per-packet VDT advancement for one frame
    pub fn setup(&mut self, total_packets: u64, last_duration: Option<u64>) {
        self.frame_packets = total_packets;
        let duration = last_duration.unwrap_or(0);

        if self.frame_packets > 0 {
            self.step = duration / total_packets;
            self.remainder = duration % total_packets;
        } else {
            self.step = 0;
            self.remainder = 0;
        }

        self.error = 0;
    }

    /// Advances VDT by `n` emitted packets
    pub fn advance(&mut self, n: u64) {
        if self.frame_packets == 0 || n == 0 {
            return;
        }

        self.value += n * self.step;

        let total_error = self.error + n * self.remainder;
        self.value += total_error / self.frame_packets;
        self.error = total_error % self.frame_packets;
    }
}

impl PartialEq for Vdt {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for Vdt {}

impl PartialOrd for Vdt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Vdt {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
