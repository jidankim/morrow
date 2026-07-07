use morrow_detection::{
    plan_list_intake_extraction, AiProvider, DetectionOutcome, ListIntakeConfidenceTier,
    ListIntakeProviderRequest, ListIntakeRouteDecision, ListIntakeSchedulingDecision,
    ProviderRouteOutcomeKind, ValidatedListIntakeExtraction,
};
use morrow_messages::{MessageEvidence, MessageSenderGroup};
use morrow_storage::{
    ListIntakeExtractionDraft, ListIntakeItemDraft, ListIntakeLocalDayWindow,
    ListIntakeProviderDiagnosticDraft, Store,
};

use super::{list_intake, storage_error, ListIntakeProfileRequest, ScanSelectedChatsError};

#[path = "list_intake_digest.rs"]
mod list_intake_digest;

const FNV_OFFSET: u32 = 2_166_136_261;
const FNV_PRIME: u32 = 16_777_619;

pub(super) struct ListIntakeRunRequest<'a, P> {
    pub(super) store: &'a Store,
    pub(super) provider: &'a P,
    pub(super) profiles: &'a [ListIntakeProfileRequest],
    pub(super) messages: &'a [MessageEvidence],
    pub(super) sender_groups: &'a [MessageSenderGroup],
    pub(super) outcomes: &'a [DetectionOutcome],
    pub(super) reference_timezone: &'a str,
}

pub(super) fn run_list_intake<P>(
    request: ListIntakeRunRequest<'_, P>,
) -> Result<(), ScanSelectedChatsError>
where
    P: AiProvider,
{
    if request.profiles.is_empty() {
        return Ok(());
    }
    for (index, message) in request.messages.iter().enumerate() {
        let scheduling = request.outcomes.get(index).map_or(
            ListIntakeSchedulingDecision::NotSchedulingOwned,
            scheduling_decision,
        );
        for profile_request in request.profiles {
            let profile = list_intake::detection_profile(profile_request);
            let route = plan_list_intake_extraction(
                &profile,
                message,
                scheduling,
                request.reference_timezone,
            );
            let ListIntakeRouteDecision::ProviderExtraction(provider_request) = route else {
                continue;
            };
            let extraction =
                match request
                    .provider
                    .extract_list_intake(ListIntakeProviderRequest::new(
                        &profile,
                        message,
                        request.reference_timezone,
                    )) {
                    Ok(extraction) => extraction,
                    Err(_error) => {
                        request
                            .store
                            .record_list_intake_provider_diagnostic(provider_diagnostic(
                                message,
                                &provider_request,
                            ))
                            .map_err(storage_error)?;
                        continue;
                    }
                };
            if should_persist(&extraction) {
                request
                    .store
                    .record_list_intake_extraction(extraction_draft(
                        message,
                        request.sender_groups,
                        extraction,
                    ))
                    .map_err(storage_error)?;
            }
        }
    }
    list_intake_digest::create_digest_reminders(
        request.store,
        request.profiles,
        request.reference_timezone,
    )?;
    Ok(())
}

fn scheduling_decision(outcome: &DetectionOutcome) -> ListIntakeSchedulingDecision {
    match outcome {
        DetectionOutcome::Candidate(_) => ListIntakeSchedulingDecision::SchedulingOwned,
        DetectionOutcome::QuietLog(_) => ListIntakeSchedulingDecision::NotSchedulingOwned,
        DetectionOutcome::CachedProviderRoute { outcome_kind, .. } => match outcome_kind {
            ProviderRouteOutcomeKind::Candidate => ListIntakeSchedulingDecision::SchedulingOwned,
            ProviderRouteOutcomeKind::QuietLog => ListIntakeSchedulingDecision::NotSchedulingOwned,
        },
    }
}

fn should_persist(extraction: &ValidatedListIntakeExtraction) -> bool {
    if extraction.items.is_empty() {
        return false;
    }
    match extraction.confidence_tier {
        ListIntakeConfidenceTier::AutoAggregate | ListIntakeConfidenceTier::Review => true,
        ListIntakeConfidenceTier::Low => false,
    }
}

fn extraction_draft(
    message: &MessageEvidence,
    sender_groups: &[MessageSenderGroup],
    extraction: ValidatedListIntakeExtraction,
) -> ListIntakeExtractionDraft {
    ListIntakeExtractionDraft {
        profile_id: extraction.profile_id,
        profile_version: extraction.profile_version,
        examples_hash: extraction.examples_hash,
        message_guid: extraction.message_guid,
        evidence_pointer: extraction.evidence_pointer,
        chat_key: format!("chat-key-{}", stable_hex(message.chat_guid.as_str())),
        sender_key: sender_key(message, sender_groups),
        window: ListIntakeLocalDayWindow {
            window_local_date: extraction.window.window_local_date,
            window_timezone: extraction.window.window_timezone,
            window_start_unix_seconds: extraction.window.window_start_unix_seconds,
        },
        confidence_tier: storage_confidence(extraction.confidence_tier),
        confidence_millis: i64::from(extraction.confidence_millis),
        items: extraction
            .items
            .into_iter()
            .map(|item| ListIntakeItemDraft {
                item_name: item.name,
                quantity: i64::from(item.quantity),
                unit: item.unit,
                category_id: item.category_id,
            })
            .collect(),
        observed_at: message.timestamp.as_i64(),
    }
}

fn storage_confidence(tier: ListIntakeConfidenceTier) -> morrow_storage::ListIntakeConfidenceTier {
    match tier {
        ListIntakeConfidenceTier::AutoAggregate => {
            morrow_storage::ListIntakeConfidenceTier::AutoAggregate
        }
        ListIntakeConfidenceTier::Review => morrow_storage::ListIntakeConfidenceTier::Review,
        ListIntakeConfidenceTier::Low => morrow_storage::ListIntakeConfidenceTier::Low,
    }
}

fn sender_key(message: &MessageEvidence, sender_groups: &[MessageSenderGroup]) -> Option<String> {
    sender_groups
        .iter()
        .find(|group| {
            group.chat_guid.as_str() == message.chat_guid.as_str()
                && group.message_guid.as_str() == message.message_guid.as_str()
        })
        .and_then(|group| {
            let key = group.sender_key();
            (!key.is_unknown()).then(|| key.as_str().to_owned())
        })
}

fn provider_diagnostic(
    message: &MessageEvidence,
    request: &morrow_detection::ListIntakeProviderExtractionRequest,
) -> ListIntakeProviderDiagnosticDraft {
    let chat_key = format!("chat-key-{}", stable_hex(message.chat_guid.as_str()));
    let message_hash = format!("message-hash-{}", stable_hex(message.message_guid.as_str()));
    ListIntakeProviderDiagnosticDraft {
        diagnostic_key: format!(
            "list-intake-provider-unavailable:{}:{}:{}:{}",
            request.profile_id,
            request.examples_hash,
            request.provider_schema_version,
            message_hash
        ),
        profile_id: request.profile_id.clone(),
        profile_version: request.profile_version.clone(),
        examples_hash: request.examples_hash.clone(),
        message_hash,
        chat_key,
        provider_prompt_version: request.provider_prompt_version.clone(),
        provider_schema_version: request.provider_schema_version.clone(),
        reason_code: "provider_unavailable".to_owned(),
        retry_state: "retry_available".to_owned(),
        created_at: message.timestamp.as_i64(),
    }
}

fn stable_hex(value: &str) -> String {
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:08x}")
}
