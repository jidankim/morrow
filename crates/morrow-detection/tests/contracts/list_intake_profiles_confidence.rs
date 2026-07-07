use std::error::Error;

use morrow_detection::{validate_list_intake_provider_output, ListIntakeConfidenceTier};

use crate::support::message;

use super::fixture::list_intake_profile;

#[test]
fn list_intake_provider_output_covers_clean_confidence_tiers() -> Result<(), Box<dyn Error>> {
    // Given
    let profile = list_intake_profile();
    let evidence = message(
        "chat-1",
        "msg-list-intake-confidence-1",
        "2 anchovies, 3 salmon",
        false,
    )?;
    let cases = [
        (
            "{\"matched\":true,\"confidence_millis\":900,\"items\":[\
             {\"name\":\"anchovies\",\"quantity\":2,\"unit\":null,\"categoryId\":\"seafood\",\
              \"evidenceText\":\"2 anchovies\"}]}",
            ListIntakeConfidenceTier::AutoAggregate,
            1,
            Some("seafood"),
        ),
        (
            "{\"matched\":true,\"confidence_millis\":700,\"items\":[\
             {\"name\":\"salmon\",\"quantity\":3,\"unit\":null,\"categoryId\":\"seafood\",\
              \"evidenceText\":\"3 salmon\"}]}",
            ListIntakeConfidenceTier::Review,
            1,
            Some("seafood"),
        ),
        (
            "{\"matched\":false,\"confidence_millis\":420,\"items\":[],\
             \"rejection_reason\":\"not_a_list\"}",
            ListIntakeConfidenceTier::Low,
            0,
            None,
        ),
    ];

    for (provider_output, expected_tier, expected_items, expected_category_id) in cases {
        // When
        let validated = validate_list_intake_provider_output(
            &profile,
            &evidence,
            "Asia/Seoul",
            provider_output,
        )?;

        // Then
        assert_eq!(validated.confidence_tier, expected_tier);
        assert_eq!(validated.items.len(), expected_items);
        if let Some(category_id) = expected_category_id {
            assert_eq!(
                validated
                    .items
                    .first()
                    .map(|item| item.category_id.as_str()),
                Some(category_id),
                "expected first validated item category for output {provider_output}"
            );
        }
    }
    println!(
        "list_intake_provider_output_covers_clean_confidence_tiers autoAggregate=cleanMatched900 review=cleanMatched700 lowNoMatch=unmatched420"
    );
    Ok(())
}
