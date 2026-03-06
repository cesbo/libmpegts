use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{
        Read,
        Write,
    },
};

use libmpegts::{
    mux::{
        Multiplexer,
        MuxFrame,
        MuxStream,
    },
    pes::PtsDts,
    psi::{
        PAT_PID,
        PatSectionRef,
        PmtSectionRef,
        Psi,
    },
    slicer::TsSlicer,
    ts::PACKET_SIZE,
};

/// Discovered elementary stream from PMT
struct StreamInfo {
    stream_type: u8,
    pid: u16,
}

/// Per-PID demux state for PES reassembly
struct DemuxStream {
    mux_index: usize,
    pes_buffer: Vec<u8>,
    is_key_frame: bool,
}

/// Parse PTS/DTS and header size from PES header bytes.
/// Returns (pts, dts, header_size) or None if invalid.
fn parse_pes_header(data: &[u8]) -> Option<(u64, Option<u64>, usize)> {
    if data.len() < 9 {
        return None;
    }

    // Verify start code prefix: 0x00 0x00 0x01
    if data[0] != 0x00 || data[1] != 0x00 || data[2] != 0x01 {
        return None;
    }

    let header_data_length = data[8] as usize;
    let header_size = 9 + header_data_length;

    if data.len() < header_size {
        return None;
    }

    let pts_dts_flags = (data[7] >> 6) & 0x03;

    let pts = if pts_dts_flags >= 2 {
        if data.len() < 14 {
            return None;
        }
        Some(parse_timestamp(&data[9 .. 14]))
    } else {
        None
    };

    let dts = if pts_dts_flags == 3 {
        if data.len() < 19 {
            return None;
        }
        Some(parse_timestamp(&data[14 .. 19]))
    } else {
        None
    };

    // PTS is required for a valid PES packet in this context
    let pts = pts?;

    Some((pts, dts, header_size))
}

/// Parse 33-bit PTS/DTS timestamp from 5 bytes of PES header
fn parse_timestamp(buf: &[u8]) -> u64 {
    let b0 = buf[0] as u64;
    let b1 = buf[1] as u64;
    let b2 = buf[2] as u64;
    let b3 = buf[3] as u64;
    let b4 = buf[4] as u64;

    ((b0 >> 1) & 0x07) << 30 | b1 << 22 | ((b2 >> 1) & 0x7F) << 15 | b3 << 7 | (b4 >> 1) & 0x7F
}

/// Returns true if stream_type is a known video or audio type
fn is_av_stream(stream_type: u8) -> bool {
    matches!(
        stream_type,
        // Video: MPEG-2, H.264, H.265, H.266
        0x02 | 0x1B | 0x24 | 0x33 |
        // Audio: MPEG-1/2, AAC, HE-AAC
        0x03 | 0x04 | 0x0F | 0x11 |
        // Private (AC-3, E-AC-3, DTS, etc.)
        0x06 | 0x81 | 0x82 | 0x83 | 0x84 | 0x87
    )
}

/// Discover streams from the first program in the TS file.
/// Returns (tsid, pnr, pmt_pid, streams).
fn discover_streams(path: &str) -> std::io::Result<(u16, u16, u16, Vec<StreamInfo>)> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; PACKET_SIZE * 1024];
    let mut slicer = TsSlicer::new();

    let mut pat_psi = Psi::default();
    let mut pmt_psi = Psi::default();

    let mut tsid = 1u16;
    let mut pnr = 0u16;
    let mut pmt_pid = 0u16;
    let mut pmt_found = false;
    let mut streams = Vec::new();

    'outer: loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        for packet in slicer.slice(&buffer[.. n]) {
            let pid = packet.pid();

            // Parse PAT to find the first program's PMT PID
            if pid == PAT_PID {
                if let Some(data) = pat_psi.assemble(&packet) {
                    if let Ok(pat) = PatSectionRef::try_from(data) {
                        tsid = pat.tsid();
                        for item in pat.items() {
                            if let Ok(item) = item {
                                if item.pnr() != 0 {
                                    pnr = item.pnr();
                                    pmt_pid = item.pid();
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Parse PMT to find elementary streams
            if pmt_pid != 0 && pid == pmt_pid {
                if let Some(data) = pmt_psi.assemble(&packet) {
                    if let Ok(pmt) = PmtSectionRef::try_from(data) {
                        for item in pmt.items() {
                            if let Ok(item) = item {
                                if is_av_stream(item.stream_type()) {
                                    streams.push(StreamInfo {
                                        stream_type: item.stream_type(),
                                        pid: item.pid(),
                                    });
                                }
                            }
                        }
                        pmt_found = true;
                        break 'outer;
                    }
                }
            }
        }
    }

    if !pmt_found {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "PMT not found in input file",
        ));
    }

    if streams.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no audio/video streams found in PMT",
        ));
    }

    Ok((tsid, pnr, pmt_pid, streams))
}

/// Flush a completed PES buffer: parse PES header, extract ES frame, push to mux.
fn flush_pes(demux: &mut DemuxStream, mux: &mut Multiplexer) {
    if demux.pes_buffer.is_empty() {
        return;
    }

    let Some((pts, dts, header_size)) = parse_pes_header(&demux.pes_buffer) else {
        demux.pes_buffer.clear();
        return;
    };

    let es_data = demux.pes_buffer[header_size ..].to_vec();
    if es_data.is_empty() {
        demux.pes_buffer.clear();
        return;
    }

    let mut pts_dts = PtsDts::new(pts);
    if let Some(dts) = dts {
        pts_dts = pts_dts.with_dts(dts);
    }

    let frame = MuxFrame::new(es_data)
        .with_key_frame(demux.is_key_frame)
        .with_pts_dts(pts_dts);

    mux.push_frame(demux.mux_index, frame);
    demux.pes_buffer.clear();
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: ts_remux <input.ts> <output.ts>");
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];

    // Pass 1: discover streams
    let (tsid, pnr, pmt_pid, streams) = discover_streams(input_path)?;

    eprintln!("TSID: {tsid}, PNR: {pnr}, PMT PID: {pmt_pid}");
    for s in &streams {
        eprintln!("  stream type=0x{:02X} pid={}", s.stream_type, s.pid);
    }

    // Setup multiplexer
    let mut mux = Multiplexer::new(tsid);
    mux.set_service(pnr, pmt_pid, None);

    let mut demux_map: HashMap<u16, DemuxStream> = HashMap::new();
    for s in &streams {
        let mux_index = mux.add_stream(MuxStream::new(s.stream_type, s.pid));
        demux_map.insert(
            s.pid,
            DemuxStream {
                mux_index,
                pes_buffer: Vec::with_capacity(256 * 1024),
                is_key_frame: false,
            },
        );
    }

    // Pass 2: demux and remux
    let mut input_file = File::open(input_path)?;
    let mut output_file = File::create(output_path)?;
    let mut input_buf = [0u8; PACKET_SIZE * 1024];
    let mut output_buf = [0u8; PACKET_SIZE * 1024];
    let mut slicer = TsSlicer::new();
    let mut frame_count: u64 = 0;

    loop {
        let n = input_file.read(&mut input_buf)?;
        if n == 0 {
            break;
        }

        for packet in slicer.slice(&input_buf[.. n]) {
            let pid = packet.pid();

            let Some(demux) = demux_map.get_mut(&pid) else {
                continue;
            };

            let Some(payload) = packet.payload() else {
                continue;
            };

            if packet.is_payload_start() {
                // Flush previous PES packet
                flush_pes(demux, &mut mux);
                frame_count += 1;

                // Check RAI for key frame detection
                demux.is_key_frame = packet
                    .adaptation_field()
                    .map(|af| af.discontinuity_indicator() || payload.len() > 0)
                    .unwrap_or(false);

                // Proper RAI detection from adaptation field
                // The adaptation_field flags byte bit 6 is random_access_indicator
                let raw = packet.as_ref();
                if raw[3] & 0x20 != 0 && raw[4] > 0 {
                    // AF present and non-empty
                    demux.is_key_frame = raw[5] & 0x40 != 0;
                } else {
                    demux.is_key_frame = false;
                }
            }

            demux.pes_buffer.extend_from_slice(payload);
        }

        // Periodically drain mux output
        loop {
            let n = mux.drain(&mut output_buf);
            if n == 0 {
                break;
            }
            output_file.write_all(&output_buf[.. n])?;
        }
    }

    // Drain remaining mux output
    loop {
        let n = mux.drain(&mut output_buf);
        if n == 0 {
            break;
        }
        output_file.write_all(&output_buf[.. n])?;
    }

    eprintln!("Done. {frame_count} frames processed.");

    Ok(())
}
