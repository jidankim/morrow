pub const LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID: &str = "uncategorized";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIntakeConfidenceTier {
    AutoAggregate,
    Review,
    Low,
}

impl ListIntakeConfidenceTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AutoAggregate => "auto_aggregate",
            Self::Review => "review",
            Self::Low => "low",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "auto_aggregate" => Ok(Self::AutoAggregate),
            "review" => Ok(Self::Review),
            "low" => Ok(Self::Low),
            other => Err(crate::StorageError::InvalidInput {
                field: "list_intake_confidence_tier",
                reason: format!("unknown confidence tier {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeLocalDayWindow {
    pub window_local_date: String,
    pub window_timezone: String,
    pub window_start_unix_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeItemDraft {
    pub item_name: String,
    pub quantity: i64,
    pub unit: Option<String>,
    pub category_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeCategoryRule {
    pub category_id: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeExtractionDraft {
    pub profile_id: String,
    pub profile_version: String,
    pub examples_hash: String,
    pub message_guid: String,
    pub evidence_pointer: String,
    pub chat_key: String,
    pub sender_key: Option<String>,
    pub window: ListIntakeLocalDayWindow,
    pub confidence_tier: ListIntakeConfidenceTier,
    pub confidence_millis: i64,
    pub items: Vec<ListIntakeItemDraft>,
    pub observed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListIntakeStorageReceipt {
    pub approved_entry_count: usize,
    pub proposal_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeProviderDiagnosticDraft {
    pub diagnostic_key: String,
    pub profile_id: String,
    pub profile_version: String,
    pub examples_hash: String,
    pub message_hash: String,
    pub chat_key: String,
    pub provider_prompt_version: String,
    pub provider_schema_version: String,
    pub reason_code: String,
    pub retry_state: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeProviderDiagnostic {
    pub profile_id: String,
    pub message_hash: String,
    pub reason_code: String,
    pub retry_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeAggregateQuery {
    pub profile_id: Option<String>,
    pub window_local_date: Option<String>,
    pub chat_key: Option<String>,
    pub sender_label: Option<String>,
    pub category_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeAggregateRow {
    pub profile_id: String,
    pub profile_version: String,
    pub examples_hash: String,
    pub window_local_date: String,
    pub window_timezone: String,
    pub window_start_unix_seconds: i64,
    pub chat_key: String,
    pub sender_label: String,
    pub category_id: String,
    pub item_name: String,
    pub unit: Option<String>,
    pub total_quantity: i64,
    pub entry_count: i64,
    pub source_proposal_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListIntakeProposalStatus {
    Pending,
    Approved,
    Rejected,
}

impl ListIntakeProposalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, crate::StorageError> {
        match raw {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            other => Err(crate::StorageError::InvalidInput {
                field: "list_intake_proposal_status",
                reason: format!("unknown proposal status {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeProposal {
    pub proposal_id: String,
    pub status: ListIntakeProposalStatus,
    pub profile_id: String,
    pub message_guid: String,
    pub sender_label: String,
    pub items: Vec<ListIntakeItemDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListIntakeReviewProposal {
    pub proposal_id: String,
    pub profile_id: String,
    pub chat_key: String,
    pub sender_label: String,
    pub items: Vec<ListIntakeItemDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListIntakeProposalDecision {
    ApproveEdited {
        proposal_id: String,
        items: Vec<ListIntakeItemDraft>,
        allowed_category_ids: Vec<String>,
        decided_at: i64,
    },
    Reject {
        proposal_id: String,
        decided_at: i64,
    },
}

pub fn assign_list_intake_category(item_name: &str, rules: &[ListIntakeCategoryRule]) -> String {
    let item_words = normalized_words(item_name);
    for rule in rules {
        if rule
            .keywords
            .iter()
            .any(|keyword| keyword_matches(&item_words, keyword))
        {
            return rule.category_id.clone();
        }
    }
    LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID.to_owned()
}

fn keyword_matches(item_words: &[String], keyword: &str) -> bool {
    let keyword_words = normalized_words(keyword);
    if keyword_words.is_empty() || keyword_words.len() > item_words.len() {
        return false;
    }
    item_words
        .windows(keyword_words.len())
        .any(|window| window == keyword_words.as_slice())
}

fn normalized_words(value: &str) -> Vec<String> {
    let normalized: String = value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect();
    normalized.split_whitespace().map(str::to_owned).collect()
}
