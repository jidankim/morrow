use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use morrow_storage::{
    ListIntakeAggregateQuery, ListIntakeAggregateRow, ListIntakeItemDraft,
    ListIntakeProposalDecision, ListIntakeReviewProposal, Store,
    LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};

use super::{
    labels::{category_label, ReportLabels},
    ListIntakeAggregateGroup, ListIntakeDecisionRequest, ListIntakeReviewCategory,
    ListIntakeReviewItem, ListIntakeReviewProposalCommand, ListIntakeReviewReport,
};

const DEFAULT_OUTPUT_POLICY: &str = "aggregateOnly";

pub(super) fn load_at(
    store_path: &Path,
    generated_at_unix_seconds: i64,
) -> Result<ListIntakeReviewReport, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    review_report(&store, generated_at_unix_seconds)
}

pub(super) fn decide_at(
    store_path: &Path,
    request: ListIntakeDecisionRequest,
    decided_at: i64,
    generated_at_unix_seconds: i64,
) -> Result<ListIntakeReviewReport, String> {
    let store = Store::open(store_path).map_err(|error| error.to_string())?;
    let decision = storage_decision(&store, request, decided_at)?;
    store
        .decide_list_intake_proposal(decision)
        .map_err(|error| error.to_string())?;
    review_report(&store, generated_at_unix_seconds)
}

fn review_report(
    store: &Store,
    generated_at_unix_seconds: i64,
) -> Result<ListIntakeReviewReport, String> {
    let aggregate_rows = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: None,
            window_local_date: None,
            chat_key: None,
            sender_label: None,
            category_id: None,
        })
        .map_err(|error| error.to_string())?;
    let proposals = store
        .list_intake_review_proposals()
        .map_err(|error| error.to_string())?;
    let labels = ReportLabels::new(&aggregate_rows, &proposals);
    Ok(ListIntakeReviewReport {
        generated_at_unix_seconds,
        aggregates: aggregate_groups(&aggregate_rows, &labels),
        proposals: proposal_commands(&proposals, &labels),
    })
}

fn storage_decision(
    store: &Store,
    request: ListIntakeDecisionRequest,
    decided_at: i64,
) -> Result<ListIntakeProposalDecision, String> {
    match request {
        ListIntakeDecisionRequest::ApproveEdited { proposal_id, items } => {
            let allowed_category_ids = allowed_category_ids(store, &proposal_id)?;
            Ok(ListIntakeProposalDecision::ApproveEdited {
                proposal_id,
                items: items.into_iter().map(ListIntakeItemDraft::from).collect(),
                allowed_category_ids,
                decided_at,
            })
        }
        ListIntakeDecisionRequest::Reject { proposal_id } => {
            Ok(ListIntakeProposalDecision::Reject {
                proposal_id,
                decided_at,
            })
        }
    }
}

fn aggregate_groups(
    rows: &[ListIntakeAggregateRow],
    labels: &ReportLabels,
) -> Vec<ListIntakeAggregateGroup> {
    let mut groups = BTreeMap::<AggregateGroupKey, Vec<ListIntakeReviewItem>>::new();
    for row in rows {
        groups
            .entry(AggregateGroupKey::from_row(row, labels))
            .or_default()
            .push(ListIntakeReviewItem::from(row));
    }
    groups
        .into_iter()
        .map(|(key, items)| ListIntakeAggregateGroup {
            profile_id: key.profile_id,
            profile_name: key.profile_name,
            output_policy: DEFAULT_OUTPUT_POLICY.to_owned(),
            local_date: key.local_date,
            chat_label: key.chat_label,
            sender_label: key.sender_label,
            category_label: key.category_label,
            items,
        })
        .collect()
}

fn proposal_commands(
    proposals: &[ListIntakeReviewProposal],
    labels: &ReportLabels,
) -> Vec<ListIntakeReviewProposalCommand> {
    proposals
        .iter()
        .map(|proposal| ListIntakeReviewProposalCommand {
            proposal_id: proposal.proposal_id.clone(),
            profile_name: labels.profile(&proposal.profile_id),
            chat_label: labels.chat(&proposal.chat_key),
            sender_label: proposal.sender_label.clone(),
            categories: category_options(&proposal.items),
            items: proposal
                .items
                .iter()
                .map(ListIntakeReviewItem::from)
                .collect(),
        })
        .collect()
}

fn category_options(items: &[ListIntakeItemDraft]) -> Vec<ListIntakeReviewCategory> {
    let mut category_ids = items
        .iter()
        .map(|item| item.category_id.as_str())
        .collect::<BTreeSet<_>>();
    category_ids.insert(LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID);
    category_ids
        .into_iter()
        .map(|category_id| ListIntakeReviewCategory {
            category_id: category_id.to_owned(),
            label: category_label(category_id),
        })
        .collect()
}

fn allowed_category_ids(store: &Store, proposal_id: &str) -> Result<Vec<String>, String> {
    let proposals = store
        .list_intake_review_proposals()
        .map_err(|error| error.to_string())?;
    let proposal = proposals
        .iter()
        .find(|proposal| proposal.proposal_id == proposal_id)
        .ok_or_else(|| "proposal_id: proposal not found".to_owned())?;
    Ok(proposal
        .items
        .iter()
        .filter(|item| item.category_id != LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID)
        .map(|item| item.category_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AggregateGroupKey {
    profile_id: String,
    profile_name: String,
    local_date: String,
    chat_label: String,
    sender_label: String,
    category_label: String,
}

impl AggregateGroupKey {
    fn from_row(row: &ListIntakeAggregateRow, labels: &ReportLabels) -> Self {
        Self {
            profile_id: row.profile_id.clone(),
            profile_name: labels.profile(&row.profile_id),
            local_date: row.window_local_date.clone(),
            chat_label: labels.chat(&row.chat_key),
            sender_label: row.sender_label.clone(),
            category_label: category_label(&row.category_id),
        }
    }
}

impl From<&ListIntakeAggregateRow> for ListIntakeReviewItem {
    fn from(row: &ListIntakeAggregateRow) -> Self {
        Self {
            item_name: row.item_name.clone(),
            quantity: row.total_quantity,
            unit: row.unit.clone(),
            category_id: row.category_id.clone(),
            category_label: category_label(&row.category_id),
        }
    }
}

impl From<&ListIntakeItemDraft> for ListIntakeReviewItem {
    fn from(item: &ListIntakeItemDraft) -> Self {
        Self {
            item_name: item.item_name.clone(),
            quantity: item.quantity,
            unit: item.unit.clone(),
            category_id: item.category_id.clone(),
            category_label: category_label(&item.category_id),
        }
    }
}

impl From<ListIntakeReviewItem> for ListIntakeItemDraft {
    fn from(item: ListIntakeReviewItem) -> Self {
        Self {
            item_name: item.item_name,
            quantity: item.quantity,
            unit: item.unit,
            category_id: item.category_id,
        }
    }
}
