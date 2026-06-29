use morrow_diagnostics::{
    TraceComponent, TraceDecision, TraceOperation, TraceOutcome, TracePrivacyTier, TraceSpan,
};
use morrow_storage::CandidateDraft;
use sha2::{Digest, Sha256};

use crate::types::ProviderIdentity;

pub(crate) struct TraceEvent<'a> {
    pub(crate) component: TraceComponent,
    pub(crate) operation: TraceOperation,
    pub(crate) decision: Option<TraceDecision>,
    pub(crate) outcome: TraceOutcome,
    pub(crate) reason_code: Option<&'a str>,
    pub(crate) confidence_millis: Option<u16>,
    pub(crate) title: Option<&'a str>,
    pub(crate) privacy_tier: TracePrivacyTier,
    pub(crate) provider: Option<&'a ProviderIdentity>,
}

impl TraceEvent<'_> {
    pub(crate) fn span(self, observed_at: &str) -> TraceSpan {
        let provider_id = self.provider.map(|provider| provider.provider_id.clone());
        let model_id = self.provider.map(|provider| provider.model_id.clone());
        let template_version = self
            .provider
            .map(|provider| provider.prompt_version.clone());
        TraceSpan {
            component: self.component,
            operation: self.operation,
            decision: self.decision,
            outcome: self.outcome,
            started_at: observed_at.to_owned(),
            ended_at: Some(observed_at.to_owned()),
            provider_id,
            model_id,
            template_version,
            reason_code: self.reason_code.map(str::to_owned),
            confidence_millis: self.confidence_millis,
            title_hash: self.title.map(hash_title),
            title_status: Some(title_status(self.title)),
            privacy_tier: self.privacy_tier,
            classifier_stage: None,
            router_stage: None,
            ood_score_millis: None,
            replay_run_id: None,
        }
    }
}

pub(crate) fn candidate_outcome(candidate: &CandidateDraft) -> TraceEvent<'_> {
    TraceEvent {
        component: TraceComponent::Outcome,
        operation: TraceOperation::OutcomeMaterialized,
        decision: Some(TraceDecision::Candidate),
        outcome: TraceOutcome::CandidateCreated,
        reason_code: Some("candidate_materialized"),
        confidence_millis: bounded_confidence(candidate.confidence_millis),
        title: Some(&candidate.title),
        privacy_tier: TracePrivacyTier::HashedIdentifier,
        provider: None,
    }
}

pub(crate) fn bounded_confidence(confidence_millis: i64) -> Option<u16> {
    u16::try_from(confidence_millis).ok()
}

fn title_status(title: Option<&str>) -> String {
    if title.is_some() {
        "hashed".to_owned()
    } else {
        "absent".to_owned()
    }
}

fn hash_title(title: &str) -> String {
    let digest = Sha256::digest(title.as_bytes());
    format!("sha256:{digest:x}")
}
