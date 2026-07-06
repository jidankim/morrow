use super::CandidateKind;

pub const PROVIDER_ROUTE_NATIVE_CANDIDATE_TITLE: &str = "Messages event candidate";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteSourceExcerptPolicy {
    Include,
    Hide,
}

impl ProviderRouteSourceExcerptPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Hide => "hide",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "include" => Ok(Self::Include),
            "hide" => Ok(Self::Hide),
            other => Err(crate::StorageError::InvalidInput {
                field: "source_excerpt_policy",
                reason: format!("unknown source excerpt policy {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteOutcomeKind {
    Candidate,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRouteRecordStatus {
    Recorded,
    SkippedProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteCandidate {
    pub kind: CandidateKind,
    pub title: String,
    pub confidence_millis: i64,
    pub normalized_time: String,
    pub evidence_excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRouteOutcome {
    Candidate(ProviderRouteCandidate),
    Quiet { quiet_reason: String },
    ProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteOutcomeDraft {
    pub route_fingerprint: String,
    pub provider_route_contract_version: String,
    pub provider_candidate_schema_version: String,
    pub evidence_payload_hash: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub profile_id: String,
    pub profile_version: String,
    pub profile_schema_version: String,
    pub profile_policy_version: String,
    pub source_excerpt_policy: ProviderRouteSourceExcerptPolicy,
    pub reference_observed: String,
    pub reference_timezone: String,
    pub threshold_millis: i64,
    pub parser_route: String,
    pub outcome: ProviderRouteOutcome,
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderRouteStoredOutcome {
    Candidate(ProviderRouteCandidate),
    Quiet { quiet_reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRouteLedgerRow {
    pub route_fingerprint: String,
    pub provider_route_contract_version: String,
    pub provider_candidate_schema_version: String,
    pub evidence_payload_hash: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt_version: String,
    pub profile_id: String,
    pub profile_version: String,
    pub profile_schema_version: String,
    pub profile_policy_version: String,
    pub source_excerpt_policy: ProviderRouteSourceExcerptPolicy,
    pub reference_observed: String,
    pub reference_timezone: String,
    pub threshold_millis: i64,
    pub parser_route: String,
    pub outcome: ProviderRouteStoredOutcome,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ProviderRouteLedgerRow {
    pub const fn outcome_kind(&self) -> ProviderRouteOutcomeKind {
        match &self.outcome {
            ProviderRouteStoredOutcome::Candidate(_) => ProviderRouteOutcomeKind::Candidate,
            ProviderRouteStoredOutcome::Quiet { .. } => ProviderRouteOutcomeKind::Quiet,
        }
    }
}
