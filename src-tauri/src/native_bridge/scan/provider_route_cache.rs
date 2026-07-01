mod metadata;

use morrow_detection::{
    CachedProviderOutcome, DetectionPipelineError, ProviderRouteCache, ProviderRouteDecision,
    ProviderRouteOutcomeKind as DetectionProviderRouteOutcomeKind, ProviderRouteRequest,
    ProviderRouteWriteIntent,
};
use morrow_storage::{
    CandidateDraft, ProviderRouteCandidate, ProviderRouteLedgerRow, ProviderRouteOutcome,
    ProviderRouteOutcomeDraft, ProviderRouteSourceExcerptPolicy, ProviderRouteStoredOutcome,
    QuietLogDraft, Store,
};

use super::{
    outcome_plan::{
        ScanPersistenceIntent, ScanPersistenceIntent::Candidate, ScanPersistenceIntent::QuietLog,
    },
    storage_error, ScanSelectedChatsError,
};
use metadata::{route_metadata, NativeProviderRouteCacheError};

pub(super) const PROVIDER_ROUTE_LEDGER_CONTRACT_VERSION: &str = "provider-route-ledger-v1";
const PROVIDER_ROUTE_LEDGER_HIDDEN_EXCERPT: &str =
    "Source excerpt hidden by provider-route ledger.";

pub(super) struct NativeProviderRouteCache<'a> {
    store: &'a Store,
}

impl<'a> NativeProviderRouteCache<'a> {
    pub(super) const fn new(store: &'a Store) -> Self {
        Self { store }
    }
}

impl ProviderRouteCache for NativeProviderRouteCache<'_> {
    type Error = NativeProviderRouteCacheError;

    fn resolve_provider_route(
        &self,
        request: ProviderRouteRequest<'_>,
    ) -> Result<ProviderRouteDecision, Self::Error> {
        let metadata = route_metadata(request)?;
        match self
            .store
            .provider_route_outcome(&metadata.route_fingerprint)?
        {
            Some(row) if row_matches_intent(&row, &metadata) => {
                Ok(ProviderRouteDecision::Hit(CachedProviderOutcome {
                    route_fingerprint: row.route_fingerprint,
                    outcome_kind: detection_outcome_kind(row.outcome),
                }))
            }
            Some(row) => {
                self.store
                    .delete_provider_route_outcome(&row.route_fingerprint)?;
                Ok(ProviderRouteDecision::Miss {
                    write_intent: Some(Box::new(metadata)),
                })
            }
            None => Ok(ProviderRouteDecision::Miss {
                write_intent: Some(Box::new(metadata)),
            }),
        }
    }
}

pub(super) fn stage_provider_route_ledger_writes<'a>(
    intents: &[ScanPersistenceIntent<'a>],
    write_intents: &[Option<ProviderRouteWriteIntent>],
) -> Result<Vec<ProviderRouteOutcomeDraft>, ScanSelectedChatsError> {
    let mut staged = Vec::new();
    for intent in intents {
        match intent {
            Candidate(planned) => {
                let Some(write_intent) =
                    write_intent_for(planned.trace_group_index, write_intents)?
                else {
                    continue;
                };
                staged.push(candidate_draft(write_intent, &planned.candidate)?);
            }
            QuietLog(planned) => {
                let Some(write_intent) =
                    write_intent_for(planned.trace_group_index, write_intents)?
                else {
                    continue;
                };
                staged.push(quiet_draft(write_intent, &planned.quiet_log)?);
            }
        }
    }
    Ok(staged)
}

pub(super) fn record_provider_route_ledger_writes(
    store: &Store,
    drafts: Vec<ProviderRouteOutcomeDraft>,
) -> Result<(), ScanSelectedChatsError> {
    for draft in drafts {
        store
            .record_provider_route_outcome(draft)
            .map_err(storage_error)?;
    }
    Ok(())
}

pub(super) fn pipeline_error(
    error: DetectionPipelineError<NativeProviderRouteCacheError>,
) -> ScanSelectedChatsError {
    match error {
        DetectionPipelineError::ProviderRouteCache(cache_error) => match cache_error {
            NativeProviderRouteCacheError::Storage(error) => storage_error(error),
            NativeProviderRouteCacheError::ProviderContract(reason)
            | NativeProviderRouteCacheError::CanonicalRouteSerialization(reason) => {
                ScanSelectedChatsError::Detection(reason.to_owned())
            }
        },
    }
}

fn write_intent_for(
    trace_group_index: Option<usize>,
    write_intents: &[Option<ProviderRouteWriteIntent>],
) -> Result<Option<&ProviderRouteWriteIntent>, ScanSelectedChatsError> {
    let Some(index) = trace_group_index else {
        return Ok(None);
    };
    write_intents.get(index).map(Option::as_ref).ok_or_else(|| {
        ScanSelectedChatsError::Detection(
            "provider route write intent missing outcome index".to_owned(),
        )
    })
}

fn candidate_draft(
    write_intent: &ProviderRouteWriteIntent,
    candidate: &CandidateDraft,
) -> Result<ProviderRouteOutcomeDraft, ScanSelectedChatsError> {
    base_draft(
        write_intent,
        ProviderRouteOutcome::Candidate(ProviderRouteCandidate {
            kind: candidate.kind,
            title: candidate.title.clone(),
            confidence_millis: candidate.confidence_millis,
            normalized_time: candidate.normalized_time.clone(),
            evidence_excerpt: PROVIDER_ROUTE_LEDGER_HIDDEN_EXCERPT.to_owned(),
        }),
        candidate.observed_at,
    )
    .map_err(storage_error)
}

fn quiet_draft(
    write_intent: &ProviderRouteWriteIntent,
    quiet_log: &QuietLogDraft,
) -> Result<ProviderRouteOutcomeDraft, ScanSelectedChatsError> {
    base_draft(
        write_intent,
        ProviderRouteOutcome::Quiet {
            quiet_reason: quiet_log.reason.clone(),
        },
        quiet_log.created_at,
    )
    .map_err(storage_error)
}

fn base_draft(
    write_intent: &ProviderRouteWriteIntent,
    outcome: ProviderRouteOutcome,
    observed_at: i64,
) -> Result<ProviderRouteOutcomeDraft, morrow_storage::StorageError> {
    Ok(ProviderRouteOutcomeDraft {
        route_fingerprint: write_intent.route_fingerprint.clone(),
        provider_route_contract_version: write_intent.provider_route_contract_version.clone(),
        provider_candidate_schema_version: write_intent.provider_candidate_schema_version.clone(),
        evidence_payload_hash: write_intent.evidence_payload_hash.clone(),
        provider_id: write_intent.provider_id.clone(),
        model_id: write_intent.model_id.clone(),
        prompt_version: write_intent.prompt_version.clone(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::parse(
            &write_intent.source_excerpt_policy,
        )?,
        reference_observed: write_intent.reference_observed.clone(),
        reference_timezone: write_intent.reference_timezone.clone(),
        threshold_millis: write_intent.threshold_millis,
        parser_route: write_intent.parser_route.clone(),
        outcome,
        observed_at,
    })
}

fn detection_outcome_kind(
    outcome: ProviderRouteStoredOutcome,
) -> DetectionProviderRouteOutcomeKind {
    match outcome {
        ProviderRouteStoredOutcome::Candidate(_) => DetectionProviderRouteOutcomeKind::Candidate,
        ProviderRouteStoredOutcome::Quiet { .. } => DetectionProviderRouteOutcomeKind::QuietLog,
    }
}

fn row_matches_intent(row: &ProviderRouteLedgerRow, intent: &ProviderRouteWriteIntent) -> bool {
    row.route_fingerprint == intent.route_fingerprint
        && row.provider_route_contract_version == intent.provider_route_contract_version
        && row.provider_candidate_schema_version == intent.provider_candidate_schema_version
        && row.evidence_payload_hash == intent.evidence_payload_hash
        && row.provider_id == intent.provider_id
        && row.model_id == intent.model_id
        && row.prompt_version == intent.prompt_version
        && row.source_excerpt_policy.as_str() == intent.source_excerpt_policy
        && row.reference_observed == intent.reference_observed
        && row.reference_timezone == intent.reference_timezone
        && row.threshold_millis == intent.threshold_millis
        && row.parser_route == intent.parser_route
}
