mod data;

use libmpegts::{
    psi::*,
    slicer::TsSlicer,
    utils::textcode::TextcodeRef,
};

/// Assembles a single EIT section from a stream of TS packets and parses it.
fn parse_eit(data: &[u8]) -> Vec<u8> {
    let mut psi = Psi::default();
    let mut section = Vec::new();
    TsSlicer::new().slice(data).for_each(|p| {
        if let Some(s) = psi.assemble(&p) {
            section = s.to_vec();
        }
    });
    section
}

/// Decodes DVB-coded text into a `String` for comparison.
fn decode(bytes: &[u8]) -> String {
    TextcodeRef::try_from(bytes)
        .expect("valid textcode")
        .to_string()
}

const EIT_4E_LANG: &str = "ita";
const EIT_4E_NAME: &str = "H264 HD 1080 24p";
const EIT_4E_TEXT: &str = "elementary video bit rate is 7.2Mbps, audio ac3 5.1, note: 24p is not currently/officially supported by DVB standards";

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
    let se = Desc4DRef::try_from(desc).expect("short_event_descriptor");
    assert_eq!(se.lang(), EIT_4E_LANG.as_bytes());
    assert_eq!(decode(se.event_name()), EIT_4E_NAME);
    assert_eq!(decode(se.text()), EIT_4E_TEXT);

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

// The following tests use real EIT data captured from the Astra 19.2°E network
// (ts-subdecode-sec.txt, dvbsnoop output) to exercise the short_event (0x4d),
// extended_event (0x4e) and content (0x54) descriptors against known values.

/// short_event_descriptor (0x4d): two scheduled events, each carrying a German
/// event name and no text, in an "EIT other TS, schedule" section (table 0x60).
#[test]
fn test_eit_short_event_descriptor() {
    let section = parse_eit(data::EIT_60_SCHEDULE);
    let eit = EitSectionRef::try_from(section.as_slice()).expect("Valid EIT section");

    assert_eq!(eit.table_id(), 0x60);
    assert_eq!(eit.service_id(), 28123);
    assert_eq!(eit.transport_stream_id(), 1101);
    assert_eq!(eit.original_network_id(), 1);

    let mut events = eit.events();

    // First event: "Crossover: Ambient".
    let event = events.next().expect("event").expect("valid event");
    assert_eq!(event.event_id(), 15528);
    assert_eq!(event.start_time(), 1082235900); // 2004-04-17 21:05:00 UTC
    assert_eq!(event.duration(), 3600); // 01:00:00
    assert_eq!(event.running_status(), 0);
    assert_eq!(event.free_ca_mode(), false);

    let descriptor = event
        .event_descriptors()
        .expect("descriptors")
        .into_iter()
        .next()
        .expect("descriptor")
        .expect("valid descriptor");
    let se = Desc4DRef::try_from(descriptor).expect("short_event_descriptor");
    assert_eq!(se.lang(), b"deu");
    assert_eq!(decode(se.event_name()), "Crossover: Ambient");
    assert_eq!(se.text(), b""); // text_length == 0
    assert_eq!(decode(se.text()), "");

    // Second event: "ARD-Nachtkonzert".
    let event = events.next().expect("event").expect("valid event");
    assert_eq!(event.event_id(), 15529);
    assert_eq!(event.start_time(), 1082239500); // 2004-04-17 22:05:00 UTC
    assert_eq!(event.duration(), 21480); // 05:58:00

    let descriptor = event
        .event_descriptors()
        .expect("descriptors")
        .into_iter()
        .next()
        .expect("descriptor")
        .expect("valid descriptor");
    let se = Desc4DRef::try_from(descriptor).expect("short_event_descriptor");
    assert_eq!(se.lang(), b"deu");
    assert_eq!(decode(se.event_name()), "ARD-Nachtkonzert");
    assert_eq!(se.text(), b"");

    assert!(events.next().is_none());
}

/// A content_descriptor (0x54) with five entries (including user-defined
/// nibbles) and an extended_event_descriptor (0x4e) with a long paragraph of
/// text, plus a short_event_descriptor (0x4d) carrying non-ASCII characters.
#[test]
fn test_eit_content_and_extended_event() {
    let section = parse_eit(data::EIT_4F_HESSEN);
    let eit = EitSectionRef::try_from(section.as_slice()).expect("Valid EIT section");

    assert_eq!(eit.service_id(), 28108);

    let event = eit
        .events()
        .next()
        .expect("event")
        .expect("valid event");
    assert_eq!(event.event_id(), 13205);
    assert_eq!(event.start_time(), 1082024100); // 2004-04-15 10:15:00 UTC
    assert_eq!(event.duration(), 1800);

    let mut short_event = None;
    let mut content = None;
    let mut extended_event = None;
    for descriptor in event.event_descriptors().expect("descriptors") {
        let descriptor = descriptor.expect("valid descriptor");
        match descriptor.tag() {
            Desc4DRef::TAG => {
                short_event = Some(Desc4DRef::try_from(descriptor).unwrap());
            }
            Desc54Ref::TAG => {
                content = Some(Desc54Ref::try_from(descriptor).unwrap());
            }
            Desc4ERef::TAG => {
                extended_event = Some(Desc4ERef::try_from(descriptor).unwrap());
            }
            _ => {}
        }
    }

    // short_event_descriptor with German umlaut and sharp-s in the text.
    let se = short_event.expect("short_event_descriptor");
    assert_eq!(decode(se.event_name()), "In Hessen unterwegs");
    assert_eq!(decode(se.text()), "Wo der weiße Flieder blüht");

    // content_descriptor with five entries.
    let cd = content.expect("content_descriptor");
    let entries: Vec<_> = cd
        .items()
        .map(|i| {
            (
                i.content_nibble_level_1(),
                i.content_nibble_level_2(),
                i.user_nibble_1(),
                i.user_nibble_2(),
            )
        })
        .collect();
    assert_eq!(entries, vec![
        (1, 5, 0, 0),   // soap/melodrama/folkloric
        (3, 0, 0, 0),   // show/game show (general)
        (11, 15, 2, 5), // user defined
        (15, 0, 8, 10), // user defined
        (15, 0, 2, 0),  // user defined
    ]);

    // extended_event_descriptor with a full paragraph of text.
    let ee = extended_event.expect("extended_event_descriptor");
    assert_eq!(ee.descriptor_number(), 0);
    assert_eq!(ee.last_descriptor_number(), 0);
    assert_eq!(ee.lang(), b"deu");
    assert_eq!(ee.items().count(), 0);
    assert_eq!(
        decode(ee.text()),
        "Frühmorgens um fünf, wenn die Händler aus dem gesamten \
         Rhein-Main-Gebiet ihre Waren bringen, beginnt auf dem Frankfurter \
         Blumengroßmarkt das hektische Tagesgeschäft. Karl-Heinz Stier und \
         Michaele Scherenberg haben sich das mal angesehen."
    );
}
