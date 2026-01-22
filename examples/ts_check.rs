use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Read,
};

use mpegts::{
    slicer::TsSlicer,
    ts::{
        PACKET_SIZE,
        PID_NULL,
    },
};

struct PidStats {
    count: u64,
    cc_errors: u64,
    last_cc: Option<u8>,
}

fn main() -> std::io::Result<()> {
    let path = env::args().nth(1).expect("Usage: ts_check <file.ts>");
    let mut file = File::open(&path)?;
    let mut buffer = [0u8; PACKET_SIZE * 1024];
    let mut stats: HashMap<u16, PidStats> = HashMap::new();
    let mut slicer = TsSlicer::new();

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }

        for packet in slicer.slice(&buffer[.. n]) {
            let pid = packet.pid();
            let entry = stats.entry(pid).or_insert(PidStats {
                count: 0,
                cc_errors: 0,
                last_cc: None,
            });
            entry.count += 1;

            if pid != PID_NULL {
                if packet.payload().is_some() {
                    let mut discontinuity_indicator = false;
                    if let Some(af) = packet.adaptation_field() {
                        discontinuity_indicator = af.discontinuity_indicator();
                    }

                    let cc = packet.cc();
                    if let Some(last) = entry.last_cc {
                        let expected = (last + 1) & 0x0F;
                        if cc != expected && !discontinuity_indicator {
                            entry.cc_errors += 1;
                        }
                    }
                    entry.last_cc = Some(cc);
                }
            }
        }
    }

    let mut pids: Vec<_> = stats.into_iter().collect();
    pids.sort_by_key(|(pid, _)| *pid);

    println!("{:>5}  {:>10}  {:>10}", "PID", "Packets", "CC Errors");
    for (pid, s) in pids {
        println!("{:>5}  {:>10}  {:>10}", pid, s.count, s.cc_errors);
    }

    Ok(())
}
