use std::collections::BTreeMap;

use morrow_storage::{
    CandidateDraft, CandidateKind, ListIntakeAggregateQuery, ListIntakeAggregateRow, Store,
};
use time::{Date, Duration, Month};

use super::{
    list_intake::{
        ListIntakeDigestDueTimeRequest, ListIntakeDigestOutputPolicyVersionRequest,
        ListIntakeOutputPolicyRequest, ListIntakeProfileRequest,
    },
    storage_error, ScanSelectedChatsError,
};

const DIGEST_CONFIDENCE_MILLIS: i64 = 900;
const OUTPUT_POLICY_VERSION: &str = "list-intake-digest-v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DigestGroup {
    profile_id: String,
    profile_version: String,
    examples_hash: String,
    window_local_date: String,
    window_timezone: String,
    window_start_unix_seconds: i64,
    chat_key: String,
    sender_label: String,
    category_id: String,
}

pub(super) fn create_digest_reminders(
    store: &Store,
    profiles: &[ListIntakeProfileRequest],
    reference_timezone: &str,
) -> Result<(), ScanSelectedChatsError> {
    for profile in profiles {
        let Some(digest) = digest_policy(profile) else {
            continue;
        };
        let rows = store
            .list_intake_aggregates(ListIntakeAggregateQuery {
                profile_id: Some(profile.profile_id.clone()),
                window_local_date: None,
                chat_key: None,
                sender_label: None,
                category_id: None,
            })
            .map_err(storage_error)?;
        for (group, rows) in grouped_rows(rows, profile) {
            let candidate = digest_candidate(profile, &group, &rows, digest, reference_timezone)?;
            store.create_candidate(candidate).map_err(storage_error)?;
        }
    }
    Ok(())
}

fn digest_policy(
    profile: &ListIntakeProfileRequest,
) -> Option<&super::list_intake::ListIntakeDigestReminderRequest> {
    if !profile.enabled {
        return None;
    }
    match profile.output_policy {
        ListIntakeOutputPolicyRequest::AggregateOnly => None,
        ListIntakeOutputPolicyRequest::DailyDigestReminder => profile.digest_reminder.as_ref(),
    }
}

fn grouped_rows(
    rows: Vec<ListIntakeAggregateRow>,
    profile: &ListIntakeProfileRequest,
) -> BTreeMap<DigestGroup, Vec<ListIntakeAggregateRow>> {
    let mut groups = BTreeMap::<DigestGroup, Vec<ListIntakeAggregateRow>>::new();
    for row in rows {
        if row.profile_version != profile.profile_version.as_str()
            || row.examples_hash != profile.examples_hash
            || row.item_name.trim().is_empty()
            || row.total_quantity <= 0
        {
            continue;
        }
        groups
            .entry(DigestGroup::from_row(&row))
            .or_default()
            .push(row);
    }
    groups
}

fn digest_candidate(
    profile: &ListIntakeProfileRequest,
    group: &DigestGroup,
    rows: &[ListIntakeAggregateRow],
    digest: &super::list_intake::ListIntakeDigestReminderRequest,
    reference_timezone: &str,
) -> Result<CandidateDraft, ScanSelectedChatsError> {
    let normalized_time = digest_due_time(group, digest, reference_timezone)?;
    let title = digest_title(profile, group, rows);
    let idempotency_key = digest_idempotency_key(group, digest.output_policy_version);
    Ok(CandidateDraft {
        kind: CandidateKind::TaskReminder,
        chat_guid: group_key(group),
        anchor_message_guid: idempotency_key,
        title: title.clone(),
        confidence_millis: DIGEST_CONFIDENCE_MILLIS,
        normalized_time,
        evidence_excerpt: format!("Approved aggregate rows: {}", item_summary(rows)),
        observed_at: group.window_start_unix_seconds.saturating_add(86_400),
    })
}

fn digest_due_time(
    group: &DigestGroup,
    digest: &super::list_intake::ListIntakeDigestReminderRequest,
    reference_timezone: &str,
) -> Result<String, ScanSelectedChatsError> {
    let date = parse_date(&group.window_local_date)?
        .checked_add(Duration::days(i64::from(digest.date_offset_days)))
        .ok_or_else(|| {
            ScanSelectedChatsError::Detection("digest reminder due date overflowed".to_owned())
        })?;
    let ListIntakeDigestDueTimeRequest::NineLocal = digest.due_time_local;
    Ok(format!(
        "{:04}-{:02}-{:02}T09:00:00[{reference_timezone}]",
        date.year(),
        u8::from(date.month()),
        date.day()
    ))
}

fn parse_date(value: &str) -> Result<Date, ScanSelectedChatsError> {
    let fields = value.split('-').collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err(ScanSelectedChatsError::Detection(
            "digest reminder aggregate date was invalid".to_owned(),
        ));
    }
    let year = parse_i32(fields[0])?;
    let month = parse_u8(fields[1])?;
    let day = parse_u8(fields[2])?;
    let month = Month::try_from(month).map_err(|_| {
        ScanSelectedChatsError::Detection("digest reminder aggregate month was invalid".to_owned())
    })?;
    Date::from_calendar_date(year, month, day).map_err(|_| {
        ScanSelectedChatsError::Detection("digest reminder aggregate date was invalid".to_owned())
    })
}

fn parse_i32(value: &str) -> Result<i32, ScanSelectedChatsError> {
    value.parse::<i32>().map_err(|_| {
        ScanSelectedChatsError::Detection("digest reminder aggregate year was invalid".to_owned())
    })
}

fn parse_u8(value: &str) -> Result<u8, ScanSelectedChatsError> {
    value.parse::<u8>().map_err(|_| {
        ScanSelectedChatsError::Detection(
            "digest reminder aggregate date part was invalid".to_owned(),
        )
    })
}

fn digest_title(
    profile: &ListIntakeProfileRequest,
    group: &DigestGroup,
    rows: &[ListIntakeAggregateRow],
) -> String {
    format!(
        "{} daily digest: {} {} - {}",
        profile.name,
        group.sender_label,
        group.category_id,
        item_summary(rows)
    )
}

fn item_summary(rows: &[ListIntakeAggregateRow]) -> String {
    rows.iter()
        .map(|row| match row.unit.as_deref() {
            Some(unit) => format!("{} {} {}", row.item_name, row.total_quantity, unit),
            None => format!("{} {}", row.item_name, row.total_quantity),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn digest_idempotency_key(
    group: &DigestGroup,
    version: ListIntakeDigestOutputPolicyVersionRequest,
) -> String {
    let ListIntakeDigestOutputPolicyVersionRequest::ListIntakeDigestV1 = version;
    format!(
        "{}|{}|{}|{}|{}|{OUTPUT_POLICY_VERSION}",
        group.profile_id,
        group.profile_version,
        group.examples_hash,
        group.window_start_unix_seconds,
        group_key(group)
    )
}

fn group_key(group: &DigestGroup) -> String {
    format!(
        "chat:{};sender:{};category:{}",
        group.chat_key, group.sender_label, group.category_id
    )
}

impl DigestGroup {
    fn from_row(row: &ListIntakeAggregateRow) -> Self {
        Self {
            profile_id: row.profile_id.clone(),
            profile_version: row.profile_version.clone(),
            examples_hash: row.examples_hash.clone(),
            window_local_date: row.window_local_date.clone(),
            window_timezone: row.window_timezone.clone(),
            window_start_unix_seconds: row.window_start_unix_seconds,
            chat_key: row.chat_key.clone(),
            sender_label: row.sender_label.clone(),
            category_id: row.category_id.clone(),
        }
    }
}

trait ListIntakeProfileVersionText {
    fn as_str(&self) -> &'static str;
}

impl ListIntakeProfileVersionText for super::list_intake::ListIntakeProfileVersionRequest {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ListIntakeV2 => "list-intake-v2",
        }
    }
}
