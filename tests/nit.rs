use mpegts::psi::*;
mod data;

#[test]
fn test_parse_nit() {
    let mut psi = Psi::default();
    psi.mux(data::NIT_DVBS);
    assert!(psi.check());

    let mut nit = Nit::default();
    nit.parse(&psi);

    assert_eq!(nit.table_id, 64);
    assert_eq!(nit.version, 11);
    assert_eq!(nit.network_id, 85);
    assert_eq!(nit.descriptors.len(), 0);
    assert_eq!(nit.items.len(), 6);

    let data: &[(u16, u16, usize)] = &[
        (8400, 318, 2),
        (12600, 318, 1),
        (700, 318, 1),
        (8900, 318, 1),
        (9400, 318, 1),
        (15600, 318, 1),
    ];

    for (item, (tsid, onid, desc_len)) in nit.items.iter().zip(data.iter()) {
        assert_eq!(item.tsid, *tsid);
        assert_eq!(item.onid, *onid);
        assert_eq!(item.descriptors.len(), *desc_len);
    }
}

#[test]
fn test_assemble_nit() {
    let mut nit = Nit::default();
    nit.table_id = 64;
    nit.version = 11;
    nit.network_id = 85;

    let data: &[(u16, u16, &[Vec<u8>])] = &[
        (
            8400,
            318,
            &[
                vec![0x43, 0x01, 0x23, 0x80, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03],
                vec![0x83, 0x0b, 0xc6, 0xc0, 0x2d]
            ]
        ),
        (
            12600,
            318,
            &[
                vec![0x43, 0x01, 0x10, 0x34, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03]
            ]
        ),
        (
            700,
            318,
            &[
                vec![0x43, 0x01, 0x13, 0x17, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03]
            ]
        ),
        (
            8900,
            318,
            &[
                vec![0x43, 0x01, 0x24, 0x76, 0x00, 0x01, 0x30, 0x86, 0x02, 0x99, 0x00, 0x03]
            ]
        ),
        (
            9400,
            318,
            &[
                vec![0x43, 0x01, 0x25, 0x97, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03]
            ]
        ),
        (
            15600,
            318,
            &[
                vec![0x43, 0x01, 0x16, 0x23, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03]
            ]
        ),
    ];

    for (tsid, onid, descs) in data.iter() {
        let mut item = NitItem::default();
        item.tsid = *tsid;
        item.onid = *onid;

        for desc in descs.iter() {
            item.descriptors.push(
                Descriptor::DescRaw(
                    DescRaw{
                        tag: desc[0],
                        data: Vec::from(&desc[1 ..])
                    }
                )
            );
        }
        nit.items.push(item);
    }

    let mut cc: u8 = 15;
    let mut nit_ts = Vec::<u8>::new();
    nit.demux(NIT_PID, &mut cc, &mut nit_ts);

    assert_eq!(data::NIT_DVBS, nit_ts.as_slice());
}
