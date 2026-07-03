use morrow_storage::validate_normalized_time;

#[test]
fn normalized_time_rejects_no_zone_offset_and_semantic_failures() {
    for value in [
        "2026-07-01T10:00:00",
        "2026-07-01T10:00:00+09:00",
        "2026-02-30T10:00:00Z",
        "2026-07-01T24:00:00Z",
    ] {
        // Given / When
        let error = validate_normalized_time(value);

        // Then
        assert!(error.is_err(), "{value}");
    }
}
