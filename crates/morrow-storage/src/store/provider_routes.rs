mod row;

use crate::sqlite_cli::sql_text;
use crate::types::{
    ProviderRouteLedgerRow, ProviderRouteOutcome, ProviderRouteOutcomeDraft,
    ProviderRouteRecordStatus,
};
use crate::validation::validate_text;
use crate::StorageError;

use super::Store;
use row::{
    parse_provider_route_row, validate_provider_route_base, validate_provider_route_candidate,
};

impl Store {
    pub fn record_provider_route_outcome(
        &self,
        draft: ProviderRouteOutcomeDraft,
    ) -> Result<ProviderRouteRecordStatus, StorageError> {
        match &draft.outcome {
            ProviderRouteOutcome::ProviderUnavailable => {
                Ok(ProviderRouteRecordStatus::SkippedProviderUnavailable)
            }
            ProviderRouteOutcome::Candidate(candidate) => {
                validate_provider_route_base(&draft)?;
                validate_provider_route_candidate(candidate)?;
                let sql = format!(
                    "INSERT INTO provider_route_outcomes
                     (route_fingerprint, provider_route_contract_version,
                      provider_candidate_schema_version, evidence_payload_hash, provider_id,
                      model_id, prompt_version, profile_id, profile_version,
                      profile_schema_version, profile_policy_version, source_excerpt_policy,
                      reference_observed, reference_timezone, threshold_millis, parser_route, outcome_kind,
                      candidate_kind, candidate_title, candidate_confidence_millis,
                      candidate_normalized_time, candidate_evidence_excerpt, quiet_reason,
                      created_at, updated_at)
                     VALUES
                     ({route_fingerprint}, {contract_version}, {candidate_schema_version},
                      {evidence_hash}, {provider_id}, {model_id}, {prompt_version},
                      {profile_id}, {profile_version}, {profile_schema_version},
                      {profile_policy_version}, {source_excerpt_policy}, {reference_observed},
                      {reference_timezone}, {threshold_millis}, {parser_route}, 'candidate', {candidate_kind},
                      {candidate_title}, {candidate_confidence_millis}, {candidate_normalized_time},
                      {candidate_evidence_excerpt}, NULL, {observed_at}, {observed_at});",
                    route_fingerprint = sql_text(&draft.route_fingerprint)?,
                    contract_version = sql_text(&draft.provider_route_contract_version)?,
                    candidate_schema_version = sql_text(&draft.provider_candidate_schema_version)?,
                    evidence_hash = sql_text(&draft.evidence_payload_hash)?,
                    provider_id = sql_text(&draft.provider_id)?,
                    model_id = sql_text(&draft.model_id)?,
                    prompt_version = sql_text(&draft.prompt_version)?,
                    profile_id = sql_text(&draft.profile_id)?,
                    profile_version = sql_text(&draft.profile_version)?,
                    profile_schema_version = sql_text(&draft.profile_schema_version)?,
                    profile_policy_version = sql_text(&draft.profile_policy_version)?,
                    source_excerpt_policy = sql_text(draft.source_excerpt_policy.as_str())?,
                    reference_observed = sql_text(&draft.reference_observed)?,
                    reference_timezone = sql_text(&draft.reference_timezone)?,
                    threshold_millis = draft.threshold_millis,
                    parser_route = sql_text(&draft.parser_route)?,
                    candidate_kind = sql_text(candidate.kind.as_str())?,
                    candidate_title = sql_text(&candidate.title)?,
                    candidate_confidence_millis = candidate.confidence_millis,
                    candidate_normalized_time = sql_text(&candidate.normalized_time)?,
                    candidate_evidence_excerpt = sql_text(&candidate.evidence_excerpt)?,
                    observed_at = draft.observed_at,
                );
                self.sqlite.execute(&sql)?;
                Ok(ProviderRouteRecordStatus::Recorded)
            }
            ProviderRouteOutcome::Quiet { quiet_reason } => {
                validate_provider_route_base(&draft)?;
                validate_text("quiet_reason", quiet_reason, 240)?;
                let sql = format!(
                    "INSERT INTO provider_route_outcomes
                     (route_fingerprint, provider_route_contract_version,
                      provider_candidate_schema_version, evidence_payload_hash, provider_id,
                      model_id, prompt_version, profile_id, profile_version,
                      profile_schema_version, profile_policy_version, source_excerpt_policy,
                      reference_observed, reference_timezone, threshold_millis, parser_route, outcome_kind,
                      candidate_kind, candidate_title, candidate_confidence_millis,
                      candidate_normalized_time, candidate_evidence_excerpt, quiet_reason,
                      created_at, updated_at)
                     VALUES
                     ({route_fingerprint}, {contract_version}, {candidate_schema_version},
                      {evidence_hash}, {provider_id}, {model_id}, {prompt_version},
                      {profile_id}, {profile_version}, {profile_schema_version},
                      {profile_policy_version}, {source_excerpt_policy}, {reference_observed},
                      {reference_timezone}, {threshold_millis}, {parser_route}, 'quiet', NULL, NULL, NULL, NULL, NULL,
                      {quiet_reason}, {observed_at}, {observed_at});",
                    route_fingerprint = sql_text(&draft.route_fingerprint)?,
                    contract_version = sql_text(&draft.provider_route_contract_version)?,
                    candidate_schema_version = sql_text(&draft.provider_candidate_schema_version)?,
                    evidence_hash = sql_text(&draft.evidence_payload_hash)?,
                    provider_id = sql_text(&draft.provider_id)?,
                    model_id = sql_text(&draft.model_id)?,
                    prompt_version = sql_text(&draft.prompt_version)?,
                    profile_id = sql_text(&draft.profile_id)?,
                    profile_version = sql_text(&draft.profile_version)?,
                    profile_schema_version = sql_text(&draft.profile_schema_version)?,
                    profile_policy_version = sql_text(&draft.profile_policy_version)?,
                    source_excerpt_policy = sql_text(draft.source_excerpt_policy.as_str())?,
                    reference_observed = sql_text(&draft.reference_observed)?,
                    reference_timezone = sql_text(&draft.reference_timezone)?,
                    threshold_millis = draft.threshold_millis,
                    parser_route = sql_text(&draft.parser_route)?,
                    quiet_reason = sql_text(quiet_reason)?,
                    observed_at = draft.observed_at,
                );
                self.sqlite.execute(&sql)?;
                Ok(ProviderRouteRecordStatus::Recorded)
            }
        }
    }

    pub fn provider_route_outcome(
        &self,
        route_fingerprint: &str,
    ) -> Result<Option<ProviderRouteLedgerRow>, StorageError> {
        validate_text("route_fingerprint", route_fingerprint, 160)?;
        let sql = format!(
            "SELECT route_fingerprint, provider_route_contract_version,
                    provider_candidate_schema_version, evidence_payload_hash, provider_id,
                    model_id, prompt_version, profile_id, profile_version,
                    profile_schema_version, profile_policy_version, source_excerpt_policy,
                    reference_observed, reference_timezone, threshold_millis, parser_route, outcome_kind,
                    candidate_kind, candidate_title, candidate_confidence_millis,
                    candidate_normalized_time, candidate_evidence_excerpt, quiet_reason,
                    created_at, updated_at
             FROM provider_route_outcomes
             WHERE route_fingerprint = {};",
            sql_text(route_fingerprint)?
        );
        let rows = self.sqlite.query_rows(&sql)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        parse_provider_route_row(row).map(Some)
    }
}
