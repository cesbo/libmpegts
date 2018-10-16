extern crate mpegts;

mod data;
use mpegts::{psi, textcode};


const PROVIDER: &str = "HTB+";


#[test]
fn test_parse_sdt() {
    let mut psi = psi::Psi::default();
    
    let mut skip = 0;
    while skip < data::SDT.len() {
        psi.mux(&data::SDT[skip ..]);
        skip += 188;
    }
    assert!(psi.check());

    let mut sdt = psi::Sdt::default();
    sdt.parse(&psi);

    assert_eq!(sdt.table_id, 0x46);
    assert_eq!(sdt.version, 4);
    assert_eq!(sdt.tsid, 1);
    assert_eq!(sdt.onid, 112);
    assert_eq!(sdt.items.len(), 15);

    let item = sdt.items.iter().next().unwrap();
    assert_eq!(item.pnr, 605);
    assert_eq!(item.eit_schedule_flag, 0);
    assert_eq!(item.eit_present_following_flag, 0);
    assert_eq!(item.running_status, 1);
    assert_eq!(item.free_ca_mode, 0);
    assert_eq!(item.descriptors.len(), 1);

    let desc =  match item.descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc48(v) => v,
        _ => unreachable!(),
    };
    assert_eq!(desc.service_type, 12);
    assert_eq!(desc.provider.to_string(), "HTB+");
    assert_eq!(desc.name.to_string(), "Neotion Update Service");
}

#[test]
fn test_assemble_sdt() {
    let mut sdt = psi::Sdt::default();
    sdt.table_id = 0x46;
    sdt.version = 4;
    sdt.tsid = 1;
    sdt.onid = 112;

    let data: &[(u16, u8, &str)] = &[
        (605, 1, "Neotion Update Service"),
        (611, 1, "Neotion CAM PRO"),
        (651, 1, "SPro16"),
        (7000, 1, "OPENTEL 1740V OTA"),
        (7010, 1, "SSU7010"),
        (8019, 1, "VAHD3100S DOWNLOAD SVC"),
        (8030, 1, "KMedia"),
        (8040, 1, "Jiuzhou 1HD SSU"),
        (8041, 4, "SSU8041"),
        (8050, 1, "DSI87"),
        (8051, 1, "SAGEM DSI74 HD"),
        (8060, 1, "Sphere1"),
        (8064, 1, "SMiT+"),
        (8070, 1, "SSU8070"),
        (8075, 1, "SSU8075")
    ];

    for d in data.iter() {
        let mut item = psi::SdtItem::default();
        item.pnr = d.0;
        item.eit_schedule_flag = 0;
        item.eit_present_following_flag = 0;
        item.running_status = d.1;
        item.free_ca_mode = 0;
        item.descriptors.push(
            psi::Descriptor::Desc48(
                psi::Desc48 {
                    service_type: 12,
                    provider: textcode::StringDVB::from_str(PROVIDER, textcode::ISO8859_5),
                    name: textcode::StringDVB::from_str(d.2, textcode::ISO8859_5)
                }
            )
        );
        sdt.items.push(item);
    }

    let mut psi_assembled = psi::Psi::default();
    sdt.assemble(&mut psi_assembled);
    // Workaround to set section_number and last_section_number values as
    // in test data, because work with them is not implemented yet.
    let size = psi_assembled.buffer.len();
    psi_assembled.buffer[6] = 0x01;
    psi_assembled.buffer[7] = 0x01;
    psi_assembled.buffer.truncate(size - 4);
    psi_assembled.finalize();

    let mut psi_check = psi::Psi::default();
    let mut skip = 0;
    while skip < data::SDT.len() {
        psi_check.mux(&data::SDT[skip ..]);
        skip += 188;
    }

    assert_eq!(psi_assembled, psi_check);
    assert_eq!(&psi_assembled.buffer[.. psi_assembled.size], &psi_check.buffer[.. psi_check.size]);
}
