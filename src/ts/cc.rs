use super::{PID_NONE, PID_NULL, TsPacketRef};

const PID_COUNT: usize = PID_NONE as usize;

pub struct CcChecker {
    last_cc: [Option<u8>; PID_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CcStatus {
    First,
    Ok,
    Discontinuity,
    Error { expected: u8, got: u8 },
}

impl CcChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, packet: &TsPacketRef) -> CcStatus {
        let pid = packet.pid();

        if pid == PID_NULL {
            return CcStatus::Ok;
        }

        let idx = usize::from(pid);

        let has_payload = packet.0[3] & 0x10 != 0; // afc x1: payload present
        let has_af = packet.0[3] & 0x20 != 0; // afc 1x: adaptation field present

        // afc 00 is reserved, such packets are discarded
        if !has_payload && !has_af {
            return CcStatus::Ok;
        }

        let cc = packet.cc();

        // 2.4.3.4, Table 2-6 "Transport stream adaptation field" H.222.0 (10/14)
        let di = packet
            .adaptation_field()
            .is_some_and(|af| af.discontinuity_indicator());

        // Stores the new CC and returns the previous one
        let last = self.last_cc[idx].replace(cc);

        if di {
            return CcStatus::Discontinuity;
        }

        match last {
            None => CcStatus::First,
            Some(last) => {
                let expected = if has_payload {
                    (last + 1) & 0x0F
                } else {
                    last
                };

                if cc == expected {
                    CcStatus::Ok
                } else {
                    CcStatus::Error { expected, got: cc }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.last_cc = [None; PID_COUNT];
    }
}

impl Default for CcChecker {
    fn default() -> Self {
        Self { last_cc: [None; PID_COUNT] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ts::{PACKET_SIZE, TsPacketMut};

    // AFC 01: payload only
    fn payload_packet(buf: &mut [u8; PACKET_SIZE], pid: u16, cc: u8) -> TsPacketRef<'_> {
        let mut packet = TsPacketMut::from(buf);
        packet.init(pid, cc);
        packet.set_payload();
        packet.into()
    }

    // AFC 10: adaptation field only
    fn af_only_packet(buf: &mut [u8; PACKET_SIZE], pid: u16, cc: u8) -> TsPacketRef<'_> {
        let mut packet = TsPacketMut::from(buf);
        packet.init(pid, cc);
        packet.set_adaptation_field(2);
        packet.into()
    }

    // AFC 11: AF + payload
    fn af_payload_packet(buf: &mut [u8; PACKET_SIZE], pid: u16, cc: u8) -> TsPacketRef<'_> {
        let mut packet = TsPacketMut::from(buf);
        packet.init(pid, cc);
        packet.set_adaptation_field(2);
        packet.set_payload();
        packet.into()
    }

    // AFC 11 + discontinuity_indicator
    fn di_packet(buf: &mut [u8; PACKET_SIZE], pid: u16, cc: u8) -> TsPacketRef<'_> {
        let mut packet = TsPacketMut::from(buf);
        packet.init(pid, cc);
        packet.set_adaptation_field(2);
        packet.set_payload();
        packet.set_discontinuity();
        packet.into()
    }

    // The first packet on a PID yields First
    #[test]
    fn test_cc_first() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
    }

    // CC increments by 1 in packets with payload (AFC 01)
    #[test]
    fn test_cc_payload_increment() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
        // Next packet on the same PID with payload, CC must increment by 1
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 3)), CcStatus::Ok);
    }

    // CC increments by 1 in packets with AF and payload (AFC 11)
    #[test]
    fn test_cc_af_payload_increment() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&af_payload_packet(&mut buf, 256, 2)), CcStatus::First);
        assert_eq!(checker.check(&af_payload_packet(&mut buf, 256, 3)), CcStatus::Ok);
    }

    // In AF-only packets (AFC 10) CC repeats the previous value
    #[test]
    fn test_cc_af_only_same() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
        // AFC == 10 check
        assert_eq!(checker.check(&af_only_packet(&mut buf, 256, 2)), CcStatus::Ok);
    }

    // CC changed in an AF-only packet - error
    #[test]
    fn test_cc_af_only_changed() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
        // Next packet is AF-only with a changed CC - must be Error
        assert_eq!(checker.check(&af_only_packet(&mut buf, 256, 3)), CcStatus::Error { expected: 2, got: 3 });
    }

    // discontinuity_indicator allows a CC jump
    #[test]
    fn test_cc_discontinuity() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
        // AF + payload packet with discontinuity_indicator, CC jumps 2 -> 9
        assert_eq!(checker.check(&di_packet(&mut buf, 256, 9)), CcStatus::Discontinuity);
        // After the jump counting continues from the new CC
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 10)), CcStatus::Ok);
    }

    // Null packets (PID_NULL) are not checked
    #[test]
    fn test_cc_null_pid() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, PID_NULL, 2)), CcStatus::Ok);
        // Null packets are ignored, so a CC jump also yields Ok
        assert_eq!(checker.check(&payload_packet(&mut buf, PID_NULL, 9)), CcStatus::Ok);
    }

    // Wrap-around 15 -> 0 is normal
    #[test]
    fn test_cc_wrap() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 15)), CcStatus::First);
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 0)), CcStatus::Ok);
    }

    // A lost packet yields a single error, then the stream is Ok again
    #[test]
    fn test_cc_lost_packet() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
        // Packet with CC = 3 is lost
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 4)), CcStatus::Error { expected: 3, got: 4 });
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 5)), CcStatus::Ok);
    }

    // Two interleaved PIDs are counted independently
    #[test]
    fn test_cc_two_pids() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 1)), CcStatus::First);
        assert_eq!(checker.check(&payload_packet(&mut buf, 257, 5)), CcStatus::First);
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::Ok);
        assert_eq!(checker.check(&payload_packet(&mut buf, 257, 6)), CcStatus::Ok);
        // PID 256: CC = 3 was expected
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 4)), CcStatus::Error { expected: 3, got: 4 });
        // The error on PID 256 does not affect PID 257
        assert_eq!(checker.check(&payload_packet(&mut buf, 257, 7)), CcStatus::Ok);
        // PID 256 counter resynced to CC = 4
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 5)), CcStatus::Ok);
    }

    // After reset() the next packet is First again
    #[test]
    fn test_cc_reset() {
        let mut checker = CcChecker::new();
        let mut buf = [0u8; PACKET_SIZE];

        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 1)), CcStatus::First);

        checker.reset();

        // Without reset CC = 2 would be Ok, after reset the packet is first again
        assert_eq!(checker.check(&payload_packet(&mut buf, 256, 2)), CcStatus::First);
    }
}
