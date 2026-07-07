use std::error::Error;

use morrow_detection::{validate_list_intake_provider_output, ListIntakeCategoryRule};
use morrow_messages::MessageTimestamp;

use super::fixture::list_intake_profile;
use crate::support::message;

#[test]
fn list_intake_provider_output_uses_reference_timezone_for_local_day_window(
) -> Result<(), Box<dyn Error>> {
    let profile = list_intake_profile();
    let mut evidence = message(
        "chat-1",
        "msg-list-intake-local-day",
        "2 anchovies, 3 salmon",
        false,
    )?;
    evidence.timestamp = MessageTimestamp::new(1_783_351_800)?;

    let validated = validate_list_intake_provider_output(
        &profile,
        &evidence,
        "Asia/Seoul",
        "{\"matched\":true,\"confidence_millis\":920,\"items\":[\
         {\"name\":\"anchovies\",\"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\
          \"evidenceText\":\"2 anchovies\"}]}",
    )?;

    assert_eq!(validated.window.window_timezone, "Asia/Seoul");
    assert_eq!(validated.window.window_local_date, "2026-07-07");
    assert_eq!(validated.window.window_start_unix_seconds, 1_783_350_000);
    println!(
        "list_intake_local_day_reference_timezone date={} start={}",
        validated.window.window_local_date, validated.window.window_start_unix_seconds
    );
    Ok(())
}

#[test]
fn uncategorized_provider_output_uses_whole_word_category_keyword_matching(
) -> Result<(), Box<dyn Error>> {
    let mut profile = list_intake_profile();
    profile.category_rules = vec![ListIntakeCategoryRule {
        category_id: "seafood".to_owned(),
        display_name: "Seafood".to_owned(),
        keywords: vec!["fish sauce".to_owned()],
    }];
    let evidence = message(
        "chat-1",
        "msg-list-intake-category-phrase",
        "2 fish sauce, 3 cowfish candy",
        false,
    )?;

    let validated = validate_list_intake_provider_output(
        &profile,
        &evidence,
        "Asia/Seoul",
        "{\"matched\":true,\"confidence_millis\":920,\"items\":[\
         {\"name\":\"fish sauce\",\"quantity\":2,\"unit\":null,\"categoryId\":\"uncategorized\",\
          \"evidenceText\":\"2 fish sauce\"},\
         {\"name\":\"cowfish candy\",\"quantity\":3,\"unit\":null,\"categoryId\":\"uncategorized\",\
          \"evidenceText\":\"3 cowfish candy\"}]}",
    )?;

    assert_eq!(
        validated
            .items
            .first()
            .map(|item| item.category_id.as_str()),
        Some("seafood")
    );
    assert_eq!(
        validated.items.get(1).map(|item| item.category_id.as_str()),
        Some("uncategorized")
    );
    println!(
        "list_intake_category_matching shared_whole_phrase=true substring_false_positive=false"
    );
    Ok(())
}
