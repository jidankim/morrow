use morrow_detection::{
    ListIntakeCategoryRule, ListIntakeChatScope, ListIntakeProfile, ListIntakeProfileKind,
};

pub fn list_intake_profile() -> ListIntakeProfile {
    ListIntakeProfile {
        enabled: true,
        profile_id: "list-intake-fishcount".to_owned(),
        profile_version: "list-intake-v2".to_owned(),
        name: "Fish count".to_owned(),
        kind: ListIntakeProfileKind::QuantityList,
        provider_prompt_version: "list-intake-v1".to_owned(),
        positive_examples: vec!["2 anchovies, 3 salmon".to_owned()],
        negative_examples: vec!["remind me to buy fish tomorrow".to_owned()],
        category_rules: vec![ListIntakeCategoryRule {
            category_id: "seafood".to_owned(),
            display_name: "Seafood".to_owned(),
            keywords: vec!["anchovies".to_owned(), "salmon".to_owned()],
        }],
        examples_hash: "list-intake-exampleshash".to_owned(),
        chat_scope: ListIntakeChatScope::AllSelectedChats,
        capture_from_scheduled_messages: false,
    }
}
