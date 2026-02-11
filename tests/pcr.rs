use libmpegts::ts;

#[test]
fn test_pcr_delta() {
    let current_pcr = 20000;
    let last_pcr = current_pcr - 10000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
}

#[test]
fn test_pcr_delta_overflow() {
    let current_pcr = 5000;
    let last_pcr = ts::PCR_MAX - 5000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
}
