mod meta;
mod quiet;
mod trace;

use morrow_messages::MessageEvidence;
use morrow_storage::{
    CandidateDraft, CandidateId, FeatureSnapshot, FeedbackEvent, FeedbackEventType,
    FeedbackLabelSource, FeedbackLabelValue, FeedbackRecordMeta, FeedbackSubjectType, Label,
    QuietLogDraft, Store,
};

pub(super) use trace::FeedbackTraceRecorder;
use trace::{candidate_route, CandidateRoute, TraceGroup};

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
    record_candidate_snapshot(CandidateSnapshot {
        store: input.store,
        candidate_id: input.candidate_id,
        candidate: input.candidate,
        message: input.message,
        route,
        trace,
        meta: &meta,
        config: input.config,
    })?;
    record_candidate_event(input.store, input.candidate_id, &meta)?;
    record_candidate_label(CandidateLabel {
        store: input.store,
        candidate_id: input.candidate_id,
        route,
        meta,
    })
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
    record_quiet_snapshot(QuietSnapshot {
        store: input.store,
        quiet_log: input.quiet_log,
        message: input.message,
        route: mapping.snapshot_route,
        trace,
        meta: &meta,
        config: input.config,
    })?;
    record_quiet_event(QuietEvent {
        store: input.store,
        quiet_log: input.quiet_log,
        subject_id: &subject_id,
        event_type: mapping.event_type,
        meta: &meta,
    })?;
    for label in mapping.labels {
        record_quiet_label(QuietLabel {
            store: input.store,
            subject_id: &subject_id,
            label: *label,
            meta: &meta,
        })?;
    }
    Ok(())
}

struct CandidateSnapshot<'a> {
    store: &'a Store,
    candidate_id: &'a CandidateId,
    candidate: &'a CandidateDraft,
    message: &'a MessageEvidence,
    route: CandidateRoute,
    trace: Option<&'a morrow_diagnostics::TraceRecord>,
    meta: &'a FeedbackRecordMeta,
    config: &'a ScanConfig,
}

fn record_candidate_snapshot(input: CandidateSnapshot<'_>) -> Result<(), ScanSelectedChatsError> {
    input
        .store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: format!("scan:snapshot:candidate:{}", input.candidate_id.as_str()),
            meta: input.meta.clone(),
            route: Some(input.route.as_snapshot_route().to_owned()),
            reason_code: input
                .trace
                .and_then(|record| record.span.reason_code.clone())
                .or_else(|| Some(input.route.as_reason_code().to_owned())),
            confidence_millis: Some(input.candidate.confidence_millis),
            participant_count: Some(i64::from(input.message.participant_count)),
            tapback_signal: Some(tapback_signal(input.message.tapback_signal).to_owned()),
            sender_signal_available: false,
            context_window_available: false,
            excerpt: Some(snapshot_excerpt(
                &input.candidate.evidence_excerpt,
                input.config,
            )),
        })
        .map_err(storage_error)
}

fn record_candidate_event(
    store: &Store,
    candidate_id: &CandidateId,
    meta: &FeedbackRecordMeta,
) -> Result<(), ScanSelectedChatsError> {
    store
        .record_feedback_event(FeedbackEvent {
            id: None,
            event_key: format!("scan:event:candidate_visible:{}", candidate_id.as_str()),
            event_type: FeedbackEventType::CandidateVisible,
            meta: meta.clone(),
        })
        .map_err(storage_error)
}

struct CandidateLabel<'a> {
    store: &'a Store,
    candidate_id: &'a CandidateId,
    route: CandidateRoute,
    meta: FeedbackRecordMeta,
}

fn record_candidate_label(input: CandidateLabel<'_>) -> Result<(), ScanSelectedChatsError> {
    input
        .store
        .record_label(Label {
            id: None,
            label_key: format!("scan:label:detection_route:{}", input.candidate_id.as_str()),
            label_value: FeedbackLabelValue::DetectionRoute(input.route.as_detection_label()),
            meta: input.meta,
        })
        .map_err(storage_error)
}

struct QuietSnapshot<'a> {
    store: &'a Store,
    quiet_log: &'a QuietLogDraft,
    message: &'a MessageEvidence,
    route: &'a str,
    trace: Option<&'a morrow_diagnostics::TraceRecord>,
    meta: &'a FeedbackRecordMeta,
    config: &'a ScanConfig,
}

fn record_quiet_snapshot(input: QuietSnapshot<'_>) -> Result<(), ScanSelectedChatsError> {
    let subject_id = quiet_subject_id(input.quiet_log);
    input
        .store
        .record_feature_snapshot(FeatureSnapshot {
            id: None,
            snapshot_key: format!("scan:snapshot:{subject_id}"),
            meta: input.meta.clone(),
            route: Some(input.route.to_owned()),
            reason_code: Some(input.quiet_log.reason.clone()),
            confidence_millis: input
                .trace
                .and_then(|record| record.span.confidence_millis.map(i64::from)),
            participant_count: Some(i64::from(input.message.participant_count)),
            tapback_signal: Some(tapback_signal(input.message.tapback_signal).to_owned()),
            sender_signal_available: false,
            context_window_available: false,
            excerpt: Some(snapshot_excerpt(&input.quiet_log.excerpt, input.config)),
        })
        .map_err(storage_error)
}

struct QuietEvent<'a> {
    store: &'a Store,
    quiet_log: &'a QuietLogDraft,
    subject_id: &'a str,
    event_type: FeedbackEventType,
    meta: &'a FeedbackRecordMeta,
}

fn record_quiet_event(input: QuietEvent<'_>) -> Result<(), ScanSelectedChatsError> {
    input
        .store
        .record_feedback_event(FeedbackEvent {
            id: None,
            event_key: format!("scan:event:{}:{}", input.subject_id, input.quiet_log.reason),
            event_type: input.event_type,
            meta: input.meta.clone(),
        })
        .map_err(storage_error)
}

struct QuietLabel<'a> {
    store: &'a Store,
    subject_id: &'a str,
    label: FeedbackLabelValue,
    meta: &'a FeedbackRecordMeta,
}

fn record_quiet_label(input: QuietLabel<'_>) -> Result<(), ScanSelectedChatsError> {
    input
        .store
        .record_label(Label {
            id: None,
            label_key: format!(
                "scan:label:{}:{}:{}",
                input.label.label_type().as_str(),
                input.label.as_str(),
                input.subject_id
            ),
            label_value: input.label,
            meta: input.meta.clone(),
        })
        .map_err(storage_error)
}
