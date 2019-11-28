
/// Returns `true` if packet has valid prefix
#[inline]
pub fn is_prefix(packet: &[u8]) -> bool {
    packet[0] == 0x00 && packet[1] == 0x00 && packet[2] == 0x01
}


/// According to Table 2-17 in ISO-13818-1
#[inline]
pub fn is_syntax_spec(packet: &[u8]) -> bool {
    match packet[3] {
        0xBC => false,  // program_stream_map
        0xBE => false,  // padding_stream
        0xBF => false,  // private_stream_2
        0xF0 => false,  // ECM
        0xF1 => false,  // EMM
        0xF2 => false,  // DSMCC_stream
        0xF8 => false,  // ITU-T Rec. H.222.1 type E
        0xFF => false,  // program_stream_directory
        _ => true,
    }
}


/// Returns `true` if PTS bit is set in the PTS_DTS_flags
#[inline]
pub fn is_pts(packet: &[u8]) -> bool {
    (packet[7] & 0x80) != 0
}


/// Returns PTS value
#[inline]
pub fn get_pts(packet: &[u8]) -> u64 {
    (u64::from(packet[ 9] & 0x0E) << 29) |
    (u64::from(packet[10]       ) << 22) |
    (u64::from(packet[11] & 0xFE) << 14) |
    (u64::from(packet[12]       ) <<  7) |
    (u64::from(packet[13]       ) >>  1)
}


/// Returns `true` if DTS bit is set in the PTS_DTS_flags
#[inline]
pub fn is_dts(packet: &[u8]) -> bool {
    (packet[7] & 0x40) != 0
}


/// Returns DTS value
#[inline]
pub fn get_dts(packet: &[u8]) -> u64 {
    (u64::from(packet[14] & 0x0E) << 29) |
    (u64::from(packet[15]       ) << 22) |
    (u64::from(packet[16] & 0xFE) << 14) |
    (u64::from(packet[17]       ) <<  7) |
    (u64::from(packet[18]       ) >>  1)
}
