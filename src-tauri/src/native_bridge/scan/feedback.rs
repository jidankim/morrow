mod meta;
mod quiet;
mod trace;

use morrow_messages::MessageEvidence;
use morrow_storage::{
    CandidateDraft, CandidateId, FeatureSnapshot, FeedbackEvent, FeedbackEventType,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackSubjectType, Label, QuietLogDraft, Store,
};

pub(super) use trace::FeedbackTraceRecorder;
use trace::{candidate_route, TraceGroup};

use super::{config::ScanConfig, storage_error, ScanSelectedChatsError};
use meta::{feedback_meta, snapshot_excerpt, tapback_signal, CandidateSubject, MetaInput};
use quiet::{quiet_mapping, quiet_subject_id};

pub(super) struct CandidateFeedback<'a> {
    pub(super) store: &'a Store,
    pub(super) candidate_id: &'a CandidateId,
    pub(super) candidate: &'a CandidateDraft,
    pub(super) message: &'a MessageEvidence,
    pub(super) trace_group: Option<&'a TraceGroup>,
    pub(super) config: &'a ScanConfig,
}

pub(super) struct QuietFeedback<'a> {
    pub(super) store: &'a Store,
    pub(super) quiet_log: &'a QuietLogDraft,
    pub(super) message: &'a MessageEvidence,
    pub(super) trace_group: Option<&'a TraceGroup>,
    pub(super) config: &'a ScanConfig,
}

pub(super) fn record_candidate_feedback(
    input: CandidateFeedback<'_>,
) -> Result<(), ScanSelectedChatsError> {
    let route = candidate_route(input.trace_group);
    let trace = input
        .trace_group
        .and_then(|group| group.candidate_record(route));
    let meta = feedback_meta(MetaInput {
        subject_type: FeedbackSubjectType::Candidate,
        subject_id: input.candidate_id.as_str(),
        candidate_id: Some(input.candidate_id.clone()),
        subject: CandidateSubject {
            chat_guid: &input.candidate.chat_guid,
            anchor_message_guid: &input.candidate.anchor_message_guid,
            created_at: input.candidate.observed_at,
        },
        label_source: FeedbackLabelSource::Provider,
        trace,
        config: input.config,
    })?;
    input
        .store
        .record_candidate_feedback_batch(
            FeatureSnapshot {
                id: None,
                snapshot_key: format!("scan:snapshot:candidate:{}", input.candidate_id.as_str()),
                meta: meta.clone(),
                route: Some(route.as_snapshot_route().to_owned()),
                reason_code: trace
                    .and_then(|record| record.span.reason_code.clone())
                    .or_else(|| Some(route.as_reason_code().to_owned())),
                confidence_millis: Some(input.candidate.confidence_millis),
                participant_count: Some(i64::from(input.message.participant_count)),
                tapback_signal: Some(tapback_signal(input.message.tapback_signal).to_owned()),
                sender_signal_available: false,
                context_window_available: false,
                excerpt: Some(snapshot_excerpt(
                    &input.candidate.evidence_excerpt,
                    input.config,
                )),
            },
            FeedbackEvent {
                id: None,
                event_key: format!(
                    "scan:event:candidate_visible:{}",
                    input.candidate_id.as_str()
                ),
                event_type: FeedbackEventType::CandidateVisible,
                meta: meta.clone(),
            },
            Label {
                id: None,
                label_key: format!("scan:label:detection_route:{}", input.candidate_id.as_str()),
                label_value: FeedbackLabelValue::DetectionRoute(route.as_detection_label()),
                meta,
            },
        )
        .map_err(storage_error)
}

pub(super) fn record_quiet_feedback(
    input: QuietFeedback<'_>,
) -> Result<(), ScanSelectedChatsError> {
    let subject_id = quiet_subject_id(input.quiet_log);
    let mapping = quiet_mapping(&input.quiet_log.reason)?;
    let trace = input
        .trace_group
        .and_then(|group| group.quiet_record(&input.quiet_log.reason));
    let meta = feedback_meta(MetaInput {
        subject_type: FeedbackSubjectType::QuietLog,
        subject_id: &subject_id,
        candidate_id: None,
        subject: CandidateSubject {
            chat_guid: &input.quiet_log.chat_guid,
            anchor_message_guid: &input.quiet_log.anchor_message_guid,
            created_at: input.quiet_log.created_at,
        },
        label_source: FeedbackLabelSource::QuietLog,
        trace,
        config: input.config,
    })?;
    input
        .store
        .record_quiet_feedback_batch(
            FeatureSnapshot {
                id: None,
                snapshot_key: format!("scan:snapshot:{subject_id}"),
                meta: meta.clone(),
                route: Some(mapping.snapshot_route.to_owned()),
                reason_code: Some(input.quiet_log.reason.clone()),
                confidence_millis: trace
                    .and_then(|record| record.span.confidence_millis.map(i64::from)),
                participant_count: Some(i64::from(input.message.participant_count)),
                tapback_signal: Some(tapback_signal(input.message.tapback_signal).to_owned()),
                sender_signal_available: false,
                context_window_available: false,
                excerpt: Some(snapshot_excerpt(&input.quiet_log.excerpt, input.config)),
            },
            FeedbackEvent {
                id: None,
                event_key: format!("scan:event:{subject_id}:{}", input.quiet_log.reason),
                event_type: mapping.event_type,
                meta: meta.clone(),
            },
            mapping
                .labels
                .iter()
                .map(|label| Label {
                    id: None,
                    label_key: format!(
                        "scan:label:{}:{}:{}",
                        label.label_type().as_str(),
                        label.as_str(),
                        subject_id
                    ),
                    label_value: *label,
                    meta: meta.clone(),
                })
                .collect(),
        )
        .map_err(storage_error)
}
