use libmpegts::ts;

#[test]
fn test_pcr_delta() {
    let current_pcr = 20000;
    let last_pcr = current_pcr - 10000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);

    assert_eq!(ts::pcr_delta(20000, 20000), 0);
}

#[test]
fn test_pcr_delta_overflow() {
    // the PCR ring is modulo PCR_NONE: from PCR_MAX - 5000 the counter takes
    // 5001 ticks to wrap to 0 and 5000 more to reach 5000
    let current_pcr = 5000;
    let last_pcr = ts::PCR_MAX - 5000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10001);

    // one tick from the last value to the wrapped zero
    assert_eq!(ts::pcr_delta(ts::PCR_MAX, 0), 1);

    // almost a full circle
    assert_eq!(ts::pcr_delta(1, 0), ts::PCR_MAX);
}
