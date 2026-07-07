use std::collections::{BTreeMap, BTreeSet};

use morrow_storage::{
    ListIntakeAggregateRow, ListIntakeReviewProposal, LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};

#[derive(Debug, Default)]
pub(super) struct ReportLabels {
    profile_labels: BTreeMap<String, String>,
    chat_labels: BTreeMap<String, String>,
}

impl ReportLabels {
    pub(super) fn new(
        rows: &[ListIntakeAggregateRow],
        proposals: &[ListIntakeReviewProposal],
    ) -> Self {
        let mut profile_ids = BTreeSet::new();
        let mut chat_keys = BTreeSet::new();
        for row in rows {
            profile_ids.insert(row.profile_id.as_str());
            chat_keys.insert(row.chat_key.as_str());
        }
        for proposal in proposals {
            profile_ids.insert(proposal.profile_id.as_str());
            chat_keys.insert(proposal.chat_key.as_str());
        }
        Self {
            profile_labels: ordinal_labels(profile_ids, "Profile"),
            chat_labels: ordinal_labels(chat_keys, "Chat"),
        }
    }

    pub(super) fn profile(&self, profile_id: &str) -> String {
        self.profile_labels
            .get(profile_id)
            .cloned()
            .unwrap_or_else(|| "Profile 1".to_owned())
    }

    pub(super) fn chat(&self, chat_key: &str) -> String {
        self.chat_labels
            .get(chat_key)
            .cloned()
            .unwrap_or_else(|| "Chat 1".to_owned())
    }
}

pub(super) fn category_label(category_id: &str) -> String {
    if category_id == LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID {
        return "Uncategorized".to_owned();
    }
    category_id
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(title_case_ascii)
        .collect::<Vec<_>>()
        .join(" ")
}

fn ordinal_labels(values: BTreeSet<&str>, prefix: &'static str) -> BTreeMap<String, String> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| (value.to_owned(), format!("{prefix} {}", index + 1)))
        .collect()
}

fn title_case_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first
        .to_uppercase()
        .chain(chars.flat_map(char::to_lowercase))
        .collect()
}
