use std::error::Error;
use std::fmt::{Display, Formatter};

use morrow_detection::{ProviderRouteRequest, ProviderRouteWriteIntent, SourceExcerptPolicy};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::PROVIDER_ROUTE_LEDGER_CONTRACT_VERSION;
use crate::native_bridge::provider_contract::{
    evidence_payload_text, ProviderContractError, PROVIDER_CANDIDATE_SCHEMA_VERSION,
};

pub(super) fn route_metadata(
    request: ProviderRouteRequest<'_>,
) -> Result<ProviderRouteWriteIntent, NativeProviderRouteCacheError> {
    let evidence_text = evidence_payload_text(std::slice::from_ref(request.message))
        .map_err(NativeProviderRouteCacheError::from_provider_contract)?;
    let evidence_payload_hash = hash_text(&evidence_text);
    let reference_timezone = request.config.reference.timezone.clone();
    let reference_observed = request
        .config
        .reference
        .observed
        .normalized(&reference_timezone);
    let parser_route = match request.parser_time {
        Some(parser_time) => format!(
            "parser_time:{}",
            parser_time.normalized(&reference_timezone)
        ),
        None => "none".to_owned(),
    };
    let source_excerpt_policy = source_excerpt_policy(request.config.source_excerpts);
    let canonical = CanonicalRouteMetadata {
        evidence_payload_hash: &evidence_payload_hash,
        model_id: &request.config.provider.model_id,
        parser_route: &parser_route,
        prompt_version: &request.config.provider.prompt_version,
        provider_candidate_schema_version: PROVIDER_CANDIDATE_SCHEMA_VERSION,
        provider_id: &request.config.provider.provider_id,
        provider_route_contract_version: PROVIDER_ROUTE_LEDGER_CONTRACT_VERSION,
        reference_observed: &reference_observed,
        reference_timezone: &reference_timezone,
        source_excerpt_policy,
        threshold_millis: request.config.threshold.as_i64(),
    };
    let canonical_json = serde_json::to_vec(&canonical).map_err(|_| {
        NativeProviderRouteCacheError::CanonicalRouteSerialization(
            "provider route metadata could not be serialized",
        )
    })?;
    Ok(ProviderRouteWriteIntent {
        route_fingerprint: hash_bytes(&canonical_json),
        provider_route_contract_version: PROVIDER_ROUTE_LEDGER_CONTRACT_VERSION.to_owned(),
        provider_candidate_schema_version: PROVIDER_CANDIDATE_SCHEMA_VERSION.to_owned(),
        evidence_payload_hash,
        provider_id: request.config.provider.provider_id.clone(),
        model_id: request.config.provider.model_id.clone(),
        prompt_version: request.config.provider.prompt_version.clone(),
        source_excerpt_policy: source_excerpt_policy.to_owned(),
        reference_observed,
        reference_timezone,
        threshold_millis: request.config.threshold.as_i64(),
        parser_route,
    })
}

fn source_excerpt_policy(policy: SourceExcerptPolicy) -> &'static str {
    match policy {
        SourceExcerptPolicy::Include => "include",
        SourceExcerptPolicy::Hide => "hide",
    }
}

fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalRouteMetadata<'a> {
    evidence_payload_hash: &'a str,
    model_id: &'a str,
    parser_route: &'a str,
    prompt_version: &'a str,
    provider_candidate_schema_version: &'a str,
    provider_id: &'a str,
    provider_route_contract_version: &'a str,
    reference_observed: &'a str,
    reference_timezone: &'a str,
    source_excerpt_policy: &'a str,
    threshold_millis: i64,
}

#[derive(Debug)]
pub(crate) enum NativeProviderRouteCacheError {
    Storage(morrow_storage::StorageError),
    ProviderContract(&'static str),
    CanonicalRouteSerialization(&'static str),
}

impl NativeProviderRouteCacheError {
    const fn from_provider_contract(error: ProviderContractError) -> Self {
        match error {
            ProviderContractError::EvidenceSerialization => {
                Self::ProviderContract("provider evidence payload could not be serialized")
            }
            ProviderContractError::EvidenceTooLarge => {
                Self::ProviderContract("provider evidence payload was too large")
            }
            ProviderContractError::InvalidCandidate { reason } => Self::ProviderContract(reason),
        }
    }
}

impl From<morrow_storage::StorageError> for NativeProviderRouteCacheError {
    fn from(error: morrow_storage::StorageError) -> Self {
        Self::Storage(error)
    }
}

impl Display for NativeProviderRouteCacheError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::ProviderContract(reason) | Self::CanonicalRouteSerialization(reason) => {
                formatter.write_str(reason)
            }
        }
    }
}

impl Error for NativeProviderRouteCacheError {}
