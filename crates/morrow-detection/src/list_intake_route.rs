use morrow_messages::MessageEvidence;

use super::{
    ListIntakeChatScope, ListIntakeProfile, ListIntakeProviderExtractionRequest,
    ListIntakeRouteDecision, ListIntakeSchedulingDecision, LIST_INTAKE_PROVIDER_SCHEMA_VERSION,
};

pub fn plan_list_intake_extraction(
    profile: &ListIntakeProfile,
    evidence: &MessageEvidence,
    scheduling_decision: ListIntakeSchedulingDecision,
    reference_timezone: &str,
) -> ListIntakeRouteDecision {
    if !profile.enabled {
        return ListIntakeRouteDecision::SkipProfileScope {
            reason: "disabled_profile",
        };
    }
    if !profile_scope_matches(profile, evidence) {
        return ListIntakeRouteDecision::SkipProfileScope {
            reason: "chat_scope_mismatch",
        };
    }
    if scheduling_decision == ListIntakeSchedulingDecision::SchedulingOwned
        && !profile.capture_from_scheduled_messages
    {
        return ListIntakeRouteDecision::SkipSchedulingOwned {
            reason: "scheduling_owned",
        };
    }
    if let Some(reason) = local_safety_rejection(evidence, reference_timezone) {
        return ListIntakeRouteDecision::SkipLocalSafety { reason };
    }
    ListIntakeRouteDecision::ProviderExtraction(ListIntakeProviderExtractionRequest {
        profile_id: profile.profile_id.clone(),
        profile_version: profile.profile_version.clone(),
        examples_hash: profile.examples_hash.clone(),
        evidence_pointer: evidence.evidence_pointer.clone(),
        message_guid: evidence.message_guid.as_str().to_owned(),
        reference_timezone: reference_timezone.to_owned(),
        provider_prompt_version: profile.provider_prompt_version.clone(),
        provider_schema_version: LIST_INTAKE_PROVIDER_SCHEMA_VERSION.to_owned(),
    })
}

fn profile_scope_matches(profile: &ListIntakeProfile, evidence: &MessageEvidence) -> bool {
    match &profile.chat_scope {
        ListIntakeChatScope::AllSelectedChats => true,
        ListIntakeChatScope::SelectedChatIds(chat_ids) => chat_ids
            .iter()
            .any(|chat_id| chat_id == evidence.chat_guid.as_str()),
    }
}

fn local_safety_rejection(
    evidence: &MessageEvidence,
    reference_timezone: &str,
) -> Option<&'static str> {
    let excerpt = evidence.excerpt.trim();
    if excerpt.is_empty()
        || evidence.evidence_pointer.is_empty()
        || reference_timezone.trim().is_empty()
    {
        return Some("invalid_evidence");
    }
    let lower = excerpt.to_ascii_lowercase();
    if lower.contains("http://") || lower.contains("https://") || lower.contains("www.") {
        return Some("url_like");
    }
    if lower.split_whitespace().any(looks_like_email) {
        return Some("email_like");
    }
    if is_money_only(excerpt) {
        return Some("money_only");
    }
    if looks_phone_like(excerpt) || contains_phone_like_token(excerpt) {
        return Some("phone_like");
    }
    if evidence.participant_count > 12 || !looks_like_quantity_list(excerpt) {
        return Some("unsupported_content");
    }
    None
}

fn looks_like_quantity_list(excerpt: &str) -> bool {
    let item_count = excerpt
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter(|item| {
            item.bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_digit())
        })
        .count();
    (1..=20).contains(&item_count)
}

fn looks_like_email(token: &str) -> bool {
    let trimmed =
        token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '@' && ch != '.');
    let Some((local, domain)) = trimmed.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn is_money_only(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with('$')
        && trimmed
            .trim_start_matches('$')
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn looks_phone_like(value: &str) -> bool {
    if looks_like_iso_date(value) {
        return false;
    }
    let digits = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    digits >= 7
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '(' | ')' | '.' | ' '))
}

fn looks_like_iso_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let year = parts.next().is_some_and(|part| part.len() == 4);
    let month = parts.next().is_some_and(|part| part.len() == 2);
    let day = parts.next().is_some_and(|part| part.len() == 2);
    year && month && day && parts.next().is_none()
}

fn contains_phone_like_token(value: &str) -> bool {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '\n'))
        .any(|token| {
            let token = token
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && !matches!(ch, '+' | '-'));
            let token = token
                .strip_prefix("tel:")
                .or_else(|| token.strip_prefix("sms:"))
                .or_else(|| token.strip_prefix("imessage:"))
                .unwrap_or(token);
            looks_phone_like(token)
        })
}
