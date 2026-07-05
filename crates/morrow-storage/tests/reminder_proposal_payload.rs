use morrow_storage::CandidateKind::{CalendarEvent, ReminderUpdate, TaskReminder};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, CapPolicy, ExternalObjectMapping,
    ExternalSource, StorageError, Store,
};

const UTC_15: &str = "2026-07-15T14:00:00Z";
const UTC_16: &str = "2026-07-16T14:00:00Z";
const UTC_17: &str = "2026-07-17T14:00:00Z";
const SEOUL_16: &str = "2026-07-16T14:00:00[Asia/Seoul]";
const PRIVATE_TITLE: &str = "private reminder +15551112222";

type CandidateFixture<'a> = (CandidateKind, &'a str, &'a str, i64, &'a str);

fn fresh_store(name: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, store)
}

fn create(store: &Store, fixture: CandidateFixture<'_>) -> CandidateId {
    let (kind, anchor, title, confidence_millis, normalized_time) = fixture;
    let draft = CandidateDraft {
        kind,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor.to_owned(),
        title: title.to_owned(),
        confidence_millis,
        normalized_time: normalized_time.to_owned(),
        evidence_excerpt: "source excerpt hidden".to_owned(),
        observed_at: 1_783_000_000 + confidence_millis,
    };
    store.create_candidate(draft).expect("create candidate")
}

fn create_invalid_time(store: &Store) -> StorageError {
    let draft = CandidateDraft {
        kind: TaskReminder,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: "malformed".to_owned(),
        title: "Malformed reminder".to_owned(),
        confidence_millis: 910,
        normalized_time: "tomorrow maybe".to_owned(),
        evidence_excerpt: "source excerpt hidden".to_owned(),
        observed_at: 1_783_000_910,
    };
    store
        .create_candidate(draft)
        .expect_err("reject malformed reminder time")
}

fn map_candidate(store: &Store, candidate_id: &CandidateId, source: ExternalSource) {
    store
        .upsert_external_mapping(ExternalObjectMapping {
            candidate_id: candidate_id.clone(),
            source,
            external_object_id: "existing-object".to_owned(),
            external_source_id: "existing-source".to_owned(),
            mapped_at: 1_783_010_001,
        })
        .expect("existing mapping")
}

#[test]
fn reminder_proposal_payload_loads_only_unmapped_creating_external_task_reminders() {
    // Given: cap-selected reminder candidates plus stale, wrong-kind, wrong-state, and mapped rows.
    let (_dir, store) = fresh_store("reminder-payload-selection.sqlite");
    let selected = create(&store, (TaskReminder, "selected", "Morrow QA", 950, UTC_15));
    let raw_title = create(
        &store,
        (TaskReminder, "private", PRIVATE_TITLE, 940, SEOUL_16),
    );
    let calendar = create(&store, (CalendarEvent, "calendar", "Calendar", 930, UTC_17));
    let queued = create(
        &store,
        (TaskReminder, "queued", "Queued reminder", 400, UTC_17),
    );
    let mapped = create(
        &store,
        (TaskReminder, "mapped", "Mapped reminder", 920, UTC_17),
    );
    let malformed_time_error = create_invalid_time(&store);
    let cap_plan = store
        .apply_visibility_caps(CapPolicy::refill_for_pending(4, 0), 1_783_010_000)
        .expect("apply visibility caps");
    map_candidate(&store, &mapped, ExternalSource::Reminders);
    assert_eq!(
        store.candidate_state(&queued).expect("queued state"),
        CandidateState::Queued
    );
    assert!(matches!(
        malformed_time_error,
        StorageError::InvalidInput {
            field: "normalized_time",
            ..
        }
    ));

    // When: storage loads Reminder payloads for selected cap ids, including stale and excluded ids.
    let mut selected_ids = cap_plan
        .visible
        .iter()
        .map(|proposal| proposal.candidate_id.clone())
        .collect::<Vec<_>>();
    selected_ids.extend([
        calendar,
        queued,
        CandidateId::from_storage("morrow_0000000000000999").expect("stale id"),
    ]);
    let payloads = store
        .reminder_proposal_payloads(&selected_ids)
        .expect("reminder replay payloads");

    // Then: only creating_external task reminders without a Reminders mapping are returned.
    assert_eq!(payloads.len(), 2);
    assert!(payloads.iter().any(|payload| {
        payload.candidate_id == selected
            && payload.kind == TaskReminder
            && payload.title == "Morrow QA"
            && payload.normalized_time == UTC_15
            && payload.source_id == "morrow-selected-reminders"
    }));
    assert!(payloads.iter().any(|payload| {
        payload.candidate_id == raw_title
            && payload.kind == TaskReminder
            && payload.title == "Messages reminder candidate"
            && payload.normalized_time == SEOUL_16
            && payload.source_id == "morrow-selected-reminders"
    }));
    let payload_debug = format!("{payloads:?}");
    assert!(
        !payload_debug.contains("private reminder"),
        "{payload_debug}"
    );
    assert!(!payload_debug.contains("+15551112222"), "{payload_debug}");
}

#[test]
fn recoverable_external_proposals_include_unmapped_reminders_without_broadening_calendar() {
    // Given: creating_external calendar and reminder candidates with source-specific mappings.
    let (_dir, store) = fresh_store("reminder-recovery-selection.sqlite");
    let calendar = create(&store, (CalendarEvent, "calendar", "Calendar", 950, UTC_15));
    let reminder = create(&store, (TaskReminder, "reminder", "Reminder", 940, UTC_16));
    let mapped_r = create(&store, (TaskReminder, "mapped-r", "Mapped", 930, UTC_17));
    let mapped_c = create(&store, (CalendarEvent, "mapped-c", "Mapped", 920, UTC_17));
    let mutation = create(
        &store,
        (ReminderUpdate, "mutation", "Mutation", 910, UTC_17),
    );
    for candidate_id in [&calendar, &reminder, &mapped_r, &mapped_c, &mutation] {
        store
            .transition_candidate(
                candidate_id,
                CandidateState::CreatingExternal,
                "cap_selected_for_external_creation",
                1_783_010_000,
            )
            .expect("transition creating_external");
    }
    map_candidate(&store, &mapped_r, ExternalSource::Reminders);
    map_candidate(&store, &mapped_c, ExternalSource::Calendar);

    // When: recovery candidates are selected from creating_external rows.
    let recoverable = store
        .recoverable_external_proposals()
        .expect("recoverable proposals");

    // Then: Calendar filtering remains Calendar-specific and Reminders uses Reminders mappings.
    assert_eq!(recoverable.len(), 2);
    assert!(recoverable
        .iter()
        .any(|proposal| { proposal.candidate_id == calendar && proposal.kind == CalendarEvent }));
    assert!(recoverable
        .iter()
        .any(|proposal| { proposal.candidate_id == reminder && proposal.kind == TaskReminder }));
}

#[test]
fn reminder_proposal_payload_excludes_existing_reminders_mapping() {
    // Given: a task reminder in creating_external already mapped to Reminders.
    let (_dir, store) = fresh_store("reminder-payload-existing-mapping.sqlite");
    let candidate = create(&store, (TaskReminder, "mapped", "Mapped", 950, UTC_15));
    store
        .transition_candidate(
            &candidate,
            CandidateState::CreatingExternal,
            "cap_selected_for_external_creation",
            1_783_010_000,
        )
        .expect("transition creating_external");
    map_candidate(&store, &candidate, ExternalSource::Reminders);

    // When: storage loads Reminder payloads for the mapped candidate.
    let payloads = store
        .reminder_proposal_payloads(std::slice::from_ref(&candidate))
        .expect("reminder replay payloads");

    // Then: no payload is exposed for replay, so downstream creation has no storage payload to use.
    assert!(payloads.is_empty());
    println!(
        "existing_reminders_mapping payload_count={} source=reminders adapter_call_surface=not_reached_by_storage_payload",
        payloads.len()
    );
}
