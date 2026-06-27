use std::collections::BTreeMap;

use crate::{validation::validate_text, CandidateId, CandidateKind, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapPolicy {
    visible_limit: usize,
    per_chat_limit: usize,
    liked_bypass_limit: bool,
}

impl CapPolicy {
    pub const fn onboarding_backfill() -> Self {
        Self {
            visible_limit: 10,
            per_chat_limit: 5,
            liked_bypass_limit: false,
        }
    }

    pub const fn startup_catchup() -> Self {
        Self {
            visible_limit: 5,
            per_chat_limit: 5,
            liked_bypass_limit: false,
        }
    }

    pub const fn refill_for_pending(max_visible: usize, pending_count: usize) -> Self {
        Self {
            visible_limit: max_visible.saturating_sub(pending_count),
            per_chat_limit: 5,
            liked_bypass_limit: false,
        }
    }

    pub const fn live_passive(remaining_passive_slots: usize) -> Self {
        Self {
            visible_limit: remaining_passive_slots,
            per_chat_limit: 5,
            liked_bypass_limit: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedProposal {
    pub candidate_id: CandidateId,
    pub kind: CandidateKind,
    pub chat_guid: String,
    pub confidence_millis: i64,
    pub normalized_time: String,
    pub liked: bool,
}

impl QueuedProposal {
    pub fn new(
        candidate_id: &str,
        chat_guid: &str,
        confidence_millis: i64,
        normalized_time: &str,
        liked: bool,
    ) -> Result<Self, StorageError> {
        Self::with_kind(
            candidate_id,
            CandidateKind::CalendarEvent,
            chat_guid,
            confidence_millis,
            normalized_time,
            liked,
        )
    }

    pub fn with_kind(
        candidate_id: &str,
        kind: CandidateKind,
        chat_guid: &str,
        confidence_millis: i64,
        normalized_time: &str,
        liked: bool,
    ) -> Result<Self, StorageError> {
        validate_text("chat_guid", chat_guid, 240)?;
        validate_text("normalized_time", normalized_time, 80)?;
        if !(0..=1000).contains(&confidence_millis) {
            return Err(StorageError::InvalidInput {
                field: "confidence_millis",
                reason: "must be between 0 and 1000".to_owned(),
            });
        }

        Ok(Self {
            candidate_id: CandidateId::from_storage(candidate_id)?,
            kind,
            chat_guid: chat_guid.to_owned(),
            confidence_millis,
            normalized_time: normalized_time.to_owned(),
            liked,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapPlan {
    pub visible: Vec<QueuedProposal>,
    pub deferred: Vec<QueuedProposal>,
}

pub fn plan_visibility(proposals: &[QueuedProposal], policy: CapPolicy) -> CapPlan {
    let mut ranked = proposals.to_vec();
    ranked.sort_by(|left, right| {
        right
            .liked
            .cmp(&left.liked)
            .then(right.confidence_millis.cmp(&left.confidence_millis))
            .then(left.normalized_time.cmp(&right.normalized_time))
            .then(left.candidate_id.cmp(&right.candidate_id))
    });

    let mut visible = Vec::new();
    let mut deferred = Vec::new();
    let mut per_chat_counts = BTreeMap::<String, usize>::new();

    for proposal in ranked {
        let chat_count = per_chat_counts
            .get(&proposal.chat_guid)
            .copied()
            .unwrap_or_default();
        let within_total_limit =
            visible.len() < policy.visible_limit || (proposal.liked && policy.liked_bypass_limit);
        let within_chat_limit = chat_count < policy.per_chat_limit;

        if within_total_limit && within_chat_limit {
            let next_count = chat_count.saturating_add(1);
            per_chat_counts.insert(proposal.chat_guid.clone(), next_count);
            visible.push(proposal);
        } else {
            deferred.push(proposal);
        }
    }

    CapPlan { visible, deferred }
}
