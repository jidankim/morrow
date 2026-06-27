use morrow_storage::{CandidateId, CandidateKind};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn candidate_ids_are_stable_branded_and_local_for_valid_external_coordinates(
        chat in "[A-Za-z0-9;+._:-]{12,48}",
        message in "[A-Za-z0-9;+._:-]{12,48}",
        normalized in "20[2-9][0-9]-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:00Z",
    ) {
        // Given: valid external coordinates for a possible calendar event.
        let kind = CandidateKind::CalendarEvent;

        // When: the local candidate ID is derived twice.
        let first = CandidateId::derive(kind, &chat, &message, &normalized);
        let second = CandidateId::derive(kind, &chat, &message, &normalized);

        // Then: the ID is stable, branded, ASCII, and does not expose the source GUIDs.
        prop_assert_eq!(&first, &second);
        prop_assert!(first.as_str().starts_with("morrow_"));
        prop_assert_eq!(first.as_str().len(), 23);
        prop_assert!(first.as_str().is_ascii());
        prop_assert_ne!(first.as_str(), chat);
        prop_assert_ne!(first.as_str(), message);
    }
}
