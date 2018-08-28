// ISO 8859 Parser
//
// 1. Make directory: data
// 2. Download http://ftp.unicode.org/Public/MAPPINGS/ISO8859/ to the data directory
// 3. Build generator: rustc iso8859-parser.rs
// 4. Launch: ./iso8859-parser >../src/psi/textcode/data.rs

use std::io::{BufReader, BufRead};
use std::{io, fs};

const PATH: &str = "data";
const ARR_SIZE: usize = 0xFF - 0x9F;

fn read_file(path: &str, part_name: &str) -> io::Result<()> {
    let mut arr: Vec<u16> = vec![0x0000; ARR_SIZE];

    let file = fs::File::open(format!("{}/{}", PATH, path))?;

    let path = path.split(".").next().unwrap();
    let path = path.replace("-", "_");
    print!("/// {}\n", part_name);
    print!("pub static ISO{}: [u16; {}] = [", path, ARR_SIZE);
    for line in BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim_left();
        if line.starts_with("#") {
            continue;
        }
        let line = line.trim_right();
        if line.len() == 0 {
            continue;
        }
        let mut split = line.split(char::is_whitespace);
        let code = split.next().unwrap();
        let code = u8::from_str_radix(&code[2 ..], 16).unwrap();
        if code < 0xA0 {
            continue;
        }
        let code = code - 0xA0;
        let unicode = split.next().unwrap();
        let unicode = u16::from_str_radix(&unicode[2 ..], 16).unwrap();
        arr[code as usize] = unicode;
    }

    for (n, unicode) in arr.iter().enumerate() {
        if (n % 16) == 0 {
            print!("\n    ");
        } else {
            print!(" ");
        }
        print!("0x{:04x},", unicode);
    }
    print!("\n");
    println!("];\n");

    Ok(())
}

fn main() -> io::Result<()> {
    let mut arr: Vec<String> = Vec::new();

    for entry in fs::read_dir(PATH)? {
        let entry = entry?;
        let path = entry.path();

        let name = path.file_name().unwrap().to_str().unwrap();
        if name.starts_with(".") {
            continue;
        }

        arr.push(name.to_string());
    }

    let get_part = |n: &String| -> i32 {
        let n = n.split(".").next().unwrap();
        let s = n.split("-").collect::<Vec<_>>();
        let n = s.get(1).unwrap();
        i32::from_str_radix(n, 10).unwrap()
    };
    arr.sort_by(|a, b| {
        let an = get_part(a);
        let bn = get_part(b);
        an.cmp(&bn)
    });

    for i in arr.iter() {
        let part_name = match get_part(i) {
            1 => "Western European",
            2 => "Central European",
            3 => "South European",
            4 => "North European",
            5 => "Cyrillic",
            6 => "Arabic",
            7 => "Greek",
            8 => "Hebrew",
            9 => "Turkish",
            10 => "Nordic",
            11 => "Thai",
            13 => "Baltic Rim",
            14 => "Celtic",
            15 => "Western European",
            16 => "South-Eastern European",
            _ => unreachable!(),
        };
        read_file(i, part_name)?;
    }

    Ok(())
}
