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
        MuxService,
        MuxStream,
    },
    pes::PesHeaderRef,
    psi::{
        PAT_PID,
        PatSectionRef,
        PmtSectionRef,
        Psi,
    },
    slicer::TsSlicer,
    ts::PACKET_SIZE,
};

/// Per-PID demux state for PES reassembly
struct DemuxStream {
    mux_index: usize,
    pes_buffer: Vec<u8>,
    is_key_frame: bool,
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
/// Returns (tsid, service).
fn discover_streams(path: &str) -> std::io::Result<(u16, MuxService)> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; PACKET_SIZE * 1024];
    let mut slicer = TsSlicer::new();

    let mut pat_psi = Psi::default();
    let mut pmt_psi = Psi::default();

    let mut service = MuxService::default();
    let mut tsid = 1u16;
    let mut pmt_found = false;

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
                        tsid = pat.transport_stream_id();
                        for program in pat.programs() {
                            if let Ok(program) = program {
                                if program.program_number() != 0 {
                                    service.program_number = program.program_number();
                                    service.pmt_pid = program.pid();
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Parse PMT to find elementary streams
            if service.pmt_pid != 0 && pid == service.pmt_pid {
                if let Some(data) = pmt_psi.assemble(&packet) {
                    if let Ok(pmt) = PmtSectionRef::try_from(data) {
                        service.pcr_pid = pmt.pcr_pid();
                        for stream in pmt.streams() {
                            if let Ok(stream) = stream {
                                if is_av_stream(stream.stream_type()) {
                                    let mut stream_info = MuxStream {
                                        stream_type: stream.stream_type(),
                                        elementary_pid: stream.elementary_pid(),
                                        stream_descriptors: Vec::new(),
                                    };
                                    if let Some(descriptors) = stream.stream_descriptors() {
                                        let mut desc_vec = Vec::new();
                                        for desc in descriptors {
                                            if let Ok(desc) = desc {
                                                desc_vec.extend_from_slice(desc.bytes());
                                            }
                                        }
                                        stream_info.stream_descriptors = desc_vec;
                                    }
                                    service.streams.push(stream_info);
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

    if service.streams.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no audio/video streams found in PMT",
        ));
    }

    Ok((tsid, service))
}

/// Flush a completed PES buffer: parse PES header, extract ES frame, push to mux.
fn flush_pes(demux: &mut DemuxStream, mux: &mut Multiplexer) {
    let Ok(pes) = PesHeaderRef::try_from(demux.pes_buffer.as_ref()) else {
        return;
    };

    let Some(pts_dts) = pes.pts_dts() else {
        return;
    };

    let header_size = pes.header_len();
    let es_data = demux.pes_buffer[header_size ..].to_vec();
    if !es_data.is_empty() {
        mux.push_frame(
            demux.mux_index,
            MuxFrame {
                data: es_data,
                is_key_frame: demux.is_key_frame,
                pts_dts: Some(pts_dts),
            },
        );
    }

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
    let (tsid, service) = discover_streams(input_path)?;

    eprintln!(
        "TSID: {}, PNR: {}, PMT PID: {}, PCR PID: {}",
        tsid, service.program_number, service.pmt_pid, service.pcr_pid
    );
    for s in &service.streams {
        eprintln!(
            "  stream type=0x{:02X} pid={}",
            s.stream_type, s.elementary_pid
        );
    }

    // Setup multiplexer
    let mut mux = Multiplexer::new(tsid);
    mux.add_service(&service);

    let mut demux_map: HashMap<u16, DemuxStream> = HashMap::new();
    for stream in &service.streams {
        let mux_index = mux.stream_index(stream.elementary_pid).unwrap();
        demux_map.insert(
            stream.elementary_pid,
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
