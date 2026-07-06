use crate::normalized_time::validate_normalized_time;
use crate::sqlite_cli::row_value;
use crate::types::{
    CandidateKind, ProviderRouteCandidate, ProviderRouteLedgerRow, ProviderRouteOutcomeDraft,
    ProviderRouteOutcomeKind, ProviderRouteSourceExcerptPolicy, ProviderRouteStoredOutcome,
    PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE,
};
use crate::validation::validate_text;
use crate::StorageError;

pub(super) fn validate_provider_route_base(
    draft: &ProviderRouteOutcomeDraft,
) -> Result<(), StorageError> {
    validate_text("route_fingerprint", &draft.route_fingerprint, 160)?;
    validate_text(
        "provider_route_contract_version",
        &draft.provider_route_contract_version,
        120,
    )?;
    validate_text(
        "provider_candidate_schema_version",
        &draft.provider_candidate_schema_version,
        120,
    )?;
    validate_text("evidence_payload_hash", &draft.evidence_payload_hash, 160)?;
    validate_text("provider_id", &draft.provider_id, 80)?;
    validate_text("model_id", &draft.model_id, 120)?;
    validate_text("prompt_version", &draft.prompt_version, 120)?;
    validate_metadata_token("profile_id", &draft.profile_id, 80)?;
    validate_metadata_token("profile_version", &draft.profile_version, 120)?;
    validate_metadata_token("profile_schema_version", &draft.profile_schema_version, 120)?;
    validate_metadata_token("profile_policy_version", &draft.profile_policy_version, 120)?;
    validate_text("reference_observed", &draft.reference_observed, 120)?;
    validate_text("reference_timezone", &draft.reference_timezone, 80)?;
    validate_text("parser_route", &draft.parser_route, 180)?;
    if (0..=1000).contains(&draft.threshold_millis) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "threshold_millis",
            reason: "must be between 0 and 1000".to_owned(),
        })
    }
}

pub(super) fn validate_provider_route_candidate(
    candidate: &ProviderRouteCandidate,
) -> Result<(), StorageError> {
    if candidate.title != PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE {
        return Err(StorageError::PrivacyViolation {
            reason: "provider-route candidates must use the native privacy-safe title".to_owned(),
        });
    }
    validate_text("candidate_title", &candidate.title, 160)?;
    validate_normalized_time(&candidate.normalized_time)?;
    validate_provider_route_excerpt(&candidate.evidence_excerpt)?;
    if (0..=1000).contains(&candidate.confidence_millis) {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field: "candidate_confidence_millis",
            reason: "must be between 0 and 1000".to_owned(),
        })
    }
}

pub(super) fn parse_provider_route_row(
    row: &[String],
) -> Result<ProviderRouteLedgerRow, StorageError> {
    let outcome_kind = parse_provider_route_outcome_kind(row_value(row, 16, "outcome_kind")?)?;
    let outcome = match outcome_kind {
        ProviderRouteOutcomeKind::Candidate => {
            ProviderRouteStoredOutcome::Candidate(ProviderRouteCandidate {
                kind: CandidateKind::parse(row_value(row, 17, "candidate_kind")?)?,
                title: row_value(row, 18, "candidate_title")?.to_owned(),
                confidence_millis: parse_i64(
                    row_value(row, 19, "candidate_confidence_millis")?,
                    "candidate_confidence_millis",
                )?,
                normalized_time: row_value(row, 20, "candidate_normalized_time")?.to_owned(),
                evidence_excerpt: row_value(row, 21, "candidate_evidence_excerpt")?.to_owned(),
            })
        }
        ProviderRouteOutcomeKind::Quiet => ProviderRouteStoredOutcome::Quiet {
            quiet_reason: row_value(row, 22, "quiet_reason")?.to_owned(),
        },
    };
    Ok(ProviderRouteLedgerRow {
        route_fingerprint: row_value(row, 0, "route_fingerprint")?.to_owned(),
        provider_route_contract_version: row_value(row, 1, "contract_version")?.to_owned(),
        provider_candidate_schema_version: row_value(row, 2, "candidate_schema_version")?
            .to_owned(),
        evidence_payload_hash: row_value(row, 3, "evidence_payload_hash")?.to_owned(),
        provider_id: row_value(row, 4, "provider_id")?.to_owned(),
        model_id: row_value(row, 5, "model_id")?.to_owned(),
        prompt_version: row_value(row, 6, "prompt_version")?.to_owned(),
        profile_id: row_value(row, 7, "profile_id")?.to_owned(),
        profile_version: row_value(row, 8, "profile_version")?.to_owned(),
        profile_schema_version: row_value(row, 9, "profile_schema_version")?.to_owned(),
        profile_policy_version: row_value(row, 10, "profile_policy_version")?.to_owned(),
        source_excerpt_policy: ProviderRouteSourceExcerptPolicy::parse(row_value(
            row,
            11,
            "source_excerpt_policy",
        )?)?,
        reference_observed: row_value(row, 12, "reference_observed")?.to_owned(),
        reference_timezone: row_value(row, 13, "reference_timezone")?.to_owned(),
        threshold_millis: parse_i64(row_value(row, 14, "threshold_millis")?, "threshold_millis")?,
        parser_route: row_value(row, 15, "parser_route")?.to_owned(),
        outcome,
        created_at: parse_i64(row_value(row, 23, "created_at")?, "created_at")?,
        updated_at: parse_i64(row_value(row, 24, "updated_at")?, "updated_at")?,
    })
}

fn validate_provider_route_excerpt(value: &str) -> Result<(), StorageError> {
    validate_text("candidate_evidence_excerpt", value, 280)?;
    let lowered = value.to_ascii_lowercase();
    let header_like = lowered.contains("from:") && lowered.contains("to:");
    if value.lines().count() > 3 || header_like {
        Err(StorageError::PrivacyViolation {
            reason: "provider-route storage accepts short excerpts only".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_metadata_token(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), StorageError> {
    validate_text(field, value, max_len)?;
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        Ok(())
    } else {
        Err(StorageError::InvalidInput {
            field,
            reason: "must be a bounded metadata token".to_owned(),
        })
    }
}

fn parse_i64(raw: &str, field: &'static str) -> Result<i64, StorageError> {
    raw.parse::<i64>()
        .map_err(|err| StorageError::InvalidInput {
            field,
            reason: err.to_string(),
        })
}

fn parse_provider_route_outcome_kind(raw: &str) -> Result<ProviderRouteOutcomeKind, StorageError> {
    match raw {
        "candidate" => Ok(ProviderRouteOutcomeKind::Candidate),
        "quiet" => Ok(ProviderRouteOutcomeKind::Quiet),
        other => Err(StorageError::InvalidInput {
            field: "provider_route_outcome_kind",
            reason: format!("unknown outcome kind {other}"),
        }),
    }
}
