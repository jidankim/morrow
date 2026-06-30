use morrow_storage::{
    CandidateDraft, CandidateId, DetectionRouteLabel, DiagnosticsTraceLinkage, FeatureSnapshot,
    FeedbackEvent, FeedbackEventType, FeedbackLabelSource, FeedbackLabelValue, FeedbackPrivacyTier,
    FeedbackRecordMeta, FeedbackSourceExcerptPolicy, FeedbackSubjectType, Label,
    ProposalOutcomeLabel, QuietLogDraft, Store, SystemOutcomeLabel, FEEDBACK_EVAL_SCHEMA_VERSION,
};

const REDACTED_EXCERPT: &str = "Source excerpt hidden by settings.";

pub(crate) fn record_candidate_feedback(
    store: &Store,
    candidate: &CandidateDraft,
) -> Result<(), Box<dyn std::error::Error>> {
    let candidate_id = CandidateId::derive(
        candidate.kind,
        &candidate.chat_guid,
        &candidate.anchor_message_guid,
        &candidate.normalized_time,
    );
    let route = candidate_route(candidate);
    let meta = meta(
        FeedbackSubjectType::Candidate,
        candidate_id.as_str(),
        Some(candidate_id.clone()),
        CandidateSubject {
            chat_guid: &candidate.chat_guid,
            anchor_message_guid: &candidate.anchor_message_guid,
            created_at: candidate.observed_at,
        },
        FeedbackLabelSource::Provider,
    );

    store.record_feature_snapshot(FeatureSnapshot {
        id: None,
        snapshot_key: format!("mvp-e2e:s:c:{}", candidate.anchor_message_guid),
        meta: meta.clone(),
        route: Some(route.as_snapshot_route().to_owned()),
        reason_code: Some(route.as_reason_code().to_owned()),
        confidence_millis: Some(candidate.confidence_millis),
        participant_count: Some(2),
        tapback_signal: Some("fixture".to_owned()),
        sender_signal_available: false,
        context_window_available: false,
        excerpt: Some(REDACTED_EXCERPT.to_owned()),
    })?;
    store.record_feedback_event(FeedbackEvent {
        id: None,
        event_key: format!("mvp-e2e:e:visible:{}", candidate.anchor_message_guid),
        event_type: FeedbackEventType::CandidateVisible,
        meta: meta.clone(),
    })?;
    store.record_label(Label {
        id: None,
        label_key: format!("mvp-e2e:l:route:{}", candidate.anchor_message_guid),
        label_value: FeedbackLabelValue::DetectionRoute(route.as_detection_label()),
        meta,
    })?;
    Ok(())
}

pub(crate) fn record_quiet_feedback(
    store: &Store,
    quiet: &QuietLogDraft,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject_id = quiet_subject_id(quiet);
    let mapping = quiet_mapping(&quiet.reason)?;
    let meta = meta(
        FeedbackSubjectType::QuietLog,
        &subject_id,
        None,
        CandidateSubject {
            chat_guid: &quiet.chat_guid,
            anchor_message_guid: &quiet.anchor_message_guid,
            created_at: quiet.created_at,
        },
        FeedbackLabelSource::QuietLog,
    );

    store.record_feature_snapshot(FeatureSnapshot {
        id: None,
        snapshot_key: format!("mvp-e2e:s:q:{}", quiet.anchor_message_guid),
        meta: meta.clone(),
        route: Some(mapping.snapshot_route.to_owned()),
        reason_code: Some(quiet.reason.clone()),
        confidence_millis: None,
        participant_count: Some(2),
        tapback_signal: Some("fixture".to_owned()),
        sender_signal_available: false,
        context_window_available: false,
        excerpt: Some(REDACTED_EXCERPT.to_owned()),
    })?;
    store.record_feedback_event(FeedbackEvent {
        id: None,
        event_key: format!("mvp-e2e:e:q:{}:{}", quiet.anchor_message_guid, quiet.reason),
        event_type: mapping.event_type,
        meta: meta.clone(),
    })?;
    for label in mapping.labels {
        store.record_label(Label {
            id: None,
            label_key: format!(
                "mvp-e2e:l:{}:{}:{}",
                label.label_type().as_str(),
                label.as_str(),
                quiet.anchor_message_guid
            ),
            label_value: *label,
            meta: meta.clone(),
        })?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum CandidateRoute {
    Deterministic,
    Provider,
}

impl CandidateRoute {
    const fn as_detection_label(self) -> DetectionRouteLabel {
        match self {
            Self::Deterministic => DetectionRouteLabel::DeterministicCandidate,
            Self::Provider => DetectionRouteLabel::ProviderCandidate,
        }
    }

    const fn as_snapshot_route(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic_candidate",
            Self::Provider => "provider_candidate",
        }
    }

    const fn as_reason_code(self) -> &'static str {
        match self {
            Self::Deterministic => "fixture_deterministic_candidate",
            Self::Provider => "fixture_provider_candidate",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidateSubject<'a> {
    chat_guid: &'a str,
    anchor_message_guid: &'a str,
    created_at: i64,
}

#[derive(Debug, Clone, Copy)]
struct QuietMapping {
    event_type: FeedbackEventType,
    snapshot_route: &'static str,
    labels: &'static [FeedbackLabelValue],
}

fn candidate_route(candidate: &CandidateDraft) -> CandidateRoute {
    match candidate.anchor_message_guid.as_str() {
        "msg-reminder-date-only" => CandidateRoute::Provider,
        _ => CandidateRoute::Deterministic,
    }
}

fn quiet_mapping(reason: &str) -> Result<QuietMapping, Box<dyn std::error::Error>> {
    match reason {
        "deterministic_stop:unsupported_broad_content" => Ok(deterministic_quiet(
            FeedbackEventType::DeterministicStopUnsupportedBroadContent,
        )),
        "deterministic_stop:invalid_evidence" => Ok(deterministic_quiet(
            FeedbackEventType::DeterministicStopInvalidEvidence,
        )),
        "deterministic_stop:no_scheduling_signal" => Ok(deterministic_quiet(
            FeedbackEventType::DeterministicStopNoSchedulingSignal,
        )),
        "deterministic_stop:past_or_invalid_time" => Ok(deterministic_quiet(
            FeedbackEventType::DeterministicStopPastOrInvalidTime,
        )),
        "confidence_below_threshold" => Ok(QuietMapping {
            event_type: FeedbackEventType::ConfidenceBelowThreshold,
            snapshot_route: "provider_rejection",
            labels: &[
                FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected),
                FeedbackLabelValue::ProposalOutcome(ProposalOutcomeLabel::Unknown),
            ],
        }),
        "provider_unavailable" => Ok(QuietMapping {
            event_type: FeedbackEventType::ProviderUnavailable,
            snapshot_route: "provider_rejection",
            labels: &[
                FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderUnavailable),
                FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedProvider),
            ],
        }),
        "provider_invalid_json" => Ok(provider_validation_quiet(
            FeedbackEventType::ProviderInvalidJson,
        )),
        "provider_schema_rejected" => Ok(provider_validation_quiet(
            FeedbackEventType::ProviderSchemaRejected,
        )),
        "provider_hallucinated_evidence" => Ok(provider_validation_quiet(
            FeedbackEventType::ProviderHallucinatedEvidence,
        )),
        "parser_provider_time_conflict" => Ok(provider_validation_quiet(
            FeedbackEventType::ParserProviderTimeConflict,
        )),
        other => Err(format!("unsupported quiet feedback reason: {other}").into()),
    }
}

const fn deterministic_quiet(event_type: FeedbackEventType) -> QuietMapping {
    QuietMapping {
        event_type,
        snapshot_route: "deterministic_stop",
        labels: &[FeedbackLabelValue::DetectionRoute(
            DetectionRouteLabel::QuietStop,
        )],
    }
}

const fn provider_validation_quiet(event_type: FeedbackEventType) -> QuietMapping {
    QuietMapping {
        event_type,
        snapshot_route: "provider_rejection",
        labels: &[
            FeedbackLabelValue::DetectionRoute(DetectionRouteLabel::ProviderRejected),
            FeedbackLabelValue::SystemOutcome(SystemOutcomeLabel::FailedValidation),
        ],
    }
}

fn meta(
    subject_type: FeedbackSubjectType,
    subject_id: &str,
    candidate_id: Option<CandidateId>,
    subject: CandidateSubject<'_>,
    label_source: FeedbackLabelSource,
) -> FeedbackRecordMeta {
    FeedbackRecordMeta {
        schema_version: FEEDBACK_EVAL_SCHEMA_VERSION,
        subject_type,
        subject_id: subject_id.to_owned(),
        candidate_id,
        chat_guid: subject.chat_guid.to_owned(),
        anchor_message_guid: subject.anchor_message_guid.to_owned(),
        diagnostics: DiagnosticsTraceLinkage {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            chat_hash: None,
            message_hash: None,
        },
        provider_model_prompt_version_id: None,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::Hide,
        label_source,
        privacy_tier: FeedbackPrivacyTier::LocalPrivate,
        privacy_metadata_json: "{\"fixture\":\"mvp_e2e\"}".to_owned(),
        created_at: subject.created_at,
        expires_at: None,
    }
}

fn quiet_subject_id(quiet: &QuietLogDraft) -> String {
    format!(
        "quiet:{}:{}:{}",
        quiet.chat_guid, quiet.anchor_message_guid, quiet.reason
    )
}
