mod data;

use libmpegts::{
    psi::*,
    slicer::TsSlicer,
};

// const EIT_4E_LANG: &str = "ita";
// const EIT_4E_NAME: &str = "H264 HD 1080 24p";
// const EIT_4E_TEXT: &str = "elementary video bit rate is 7.2Mbps, audio ac3 5.1, note: 24p is not currently/officially supported by DVB standards";

#[test]
fn test_parse_eit_4e() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::EIT_4E).for_each(|p| {
        psi.assemble(&p);
    });
    let eit = EitSectionRef::try_from(&psi).expect("Valid EIT section");

    assert_eq!(eit.version(), 1);
    assert_eq!(eit.pnr(), 6);
    assert_eq!(eit.tsid(), 1);
    assert_eq!(eit.onid(), 1);

    let mut items = eit.items();
    let item = items
        .next()
        .expect("First EIT item")
        .expect("Valid EIT item");
    assert_eq!(item.event_id(), 1);
    assert_eq!(item.start_time(), 1296432000);
    assert_eq!(item.duration(), 72000);
    assert_eq!(item.running_status(), 4);
    assert_eq!(item.free_ca_mode(), false);

    let mut descriptors = item.descriptors().expect("Service descriptors").into_iter();
    let desc = descriptors
        .next()
        .expect("First service descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x4d);
    // assert_eq!(&desc.lang.to_string(), EIT_4E_LANG);
    // assert_eq!(&desc.name.to_string(), EIT_4E_NAME);
    // assert_eq!(&desc.text.to_string(), EIT_4E_TEXT);

    assert!(descriptors.next().is_none());
    assert!(items.next().is_none());
}

#[test]
fn test_parse_eit_50() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::EIT_50).for_each(|p| {
        psi.assemble(&p);
    });
    let eit = EitSectionRef::try_from(&psi).expect("Valid EIT section");

    assert_eq!(eit.version(), 21);
    assert_eq!(eit.pnr(), 7375);
    assert_eq!(eit.tsid(), 7400);
    assert_eq!(eit.onid(), 1);

    let mut items = eit.items();
    let item = items
        .next()
        .expect("First EIT item")
        .expect("Valid EIT item");

    assert_eq!(item.event_id(), 31948);
    assert_eq!(item.start_time(), 1534183800);
    assert_eq!(item.duration(), 1800);
    assert_eq!(item.running_status(), 0);
    assert_eq!(item.free_ca_mode(), true);

    let mut descriptors_count = 0;
    for descriptor in item.descriptors().expect("Service descriptors") {
        let _descriptor = descriptor.expect("Valid descriptor");
        descriptors_count += 1;
    }
    assert_eq!(descriptors_count, 4);

    assert!(items.next().is_none());
}
