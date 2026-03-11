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
    assert_eq!(eit.service_id(), 6);
    assert_eq!(eit.transport_stream_id(), 1);
    assert_eq!(eit.original_network_id(), 1);

    let mut events = eit.events();
    let event = events
        .next()
        .expect("First EIT event")
        .expect("Valid EIT event");
    assert_eq!(event.event_id(), 1);
    assert_eq!(event.start_time(), 1296432000);
    assert_eq!(event.duration(), 72000);
    assert_eq!(event.running_status(), 4);
    assert_eq!(event.free_ca_mode(), false);

    let mut descriptors = event
        .event_descriptors()
        .expect("Service descriptors")
        .into_iter();
    let desc = descriptors
        .next()
        .expect("First service descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x4d);
    // assert_eq!(&desc.lang.to_string(), EIT_4E_LANG);
    // assert_eq!(&desc.name.to_string(), EIT_4E_NAME);
    // assert_eq!(&desc.text.to_string(), EIT_4E_TEXT);

    assert!(descriptors.next().is_none());
    assert!(events.next().is_none());
}

#[test]
fn test_parse_eit_50() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::EIT_50).for_each(|p| {
        psi.assemble(&p);
    });
    let eit = EitSectionRef::try_from(&psi).expect("Valid EIT section");

    assert_eq!(eit.version(), 21);
    assert_eq!(eit.service_id(), 7375);
    assert_eq!(eit.transport_stream_id(), 7400);
    assert_eq!(eit.original_network_id(), 1);

    let mut events = eit.events();
    let event = events
        .next()
        .expect("First EIT event")
        .expect("Valid EIT event");

    assert_eq!(event.event_id(), 31948);
    assert_eq!(event.start_time(), 1534183800);
    assert_eq!(event.duration(), 1800);
    assert_eq!(event.running_status(), 0);
    assert_eq!(event.free_ca_mode(), true);

    let mut descriptors_count = 0;
    for descriptor in event.event_descriptors().expect("Service descriptors") {
        let _descriptor = descriptor.expect("Valid descriptor");
        descriptors_count += 1;
    }
    assert_eq!(descriptors_count, 4);

    assert!(events.next().is_none());
}
