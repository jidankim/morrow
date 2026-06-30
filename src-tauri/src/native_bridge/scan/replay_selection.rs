use std::collections::BTreeSet;

use morrow_storage::QueuedProposal;

pub(super) struct ReplaySelectionInput<'a> {
    pub(super) visible: &'a [QueuedProposal],
    pub(super) deferred: &'a [QueuedProposal],
    pub(super) recoverable: Vec<QueuedProposal>,
}

pub(super) struct ReplaySelection {
    pub(super) replay_candidates: Vec<QueuedProposal>,
    pub(super) pending_proposal_count: usize,
}

pub(super) fn select_replay_candidates(input: ReplaySelectionInput<'_>) -> ReplaySelection {
    let ReplaySelectionInput {
        visible,
        deferred,
        recoverable,
    } = input;
    let visible_candidate_ids = visible
        .iter()
        .map(|candidate| &candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    let mut replay_candidates = visible.to_vec();
    replay_candidates.extend(
        recoverable
            .into_iter()
            .filter(|candidate| !visible_candidate_ids.contains(&candidate.candidate_id)),
    );
    let pending_proposal_count = replay_candidates.len() + deferred.len();

    ReplaySelection {
        replay_candidates,
        pending_proposal_count,
    }
}
