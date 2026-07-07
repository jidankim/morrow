use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};

use super::{localize_candidate_json, ProviderContractError};

#[test]
fn provider_contract_rejects_out_of_prompt_selected_evidence_id() -> Result<(), String> {
    let evidence = (0..21).map(evidence).collect::<Result<Vec<_>, _>>()?;
    let response = r#"{"kind":"calendar_event","title":"Hidden sync",
        "confidence_millis":900,
        "normalized_time":"2026-07-08T10:00:00[Asia/Seoul]",
        "anchor_evidence_id":"evidence://selected/20",
        "evidence_ids":["evidence://selected/20"],
        "items":null}"#;

    let error = localize_candidate_json(response, &evidence)
        .expect_err("selected evidence id outside the prompt payload should be rejected");

    assert_eq!(
        error,
        ProviderContractError::InvalidCandidate {
            reason: "candidate evidence was hallucinated"
        }
    );
    Ok(())
}

fn evidence(index: usize) -> Result<MessageEvidence, String> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse("chat-a").map_err(|error| error.to_string())?,
        message_guid: MessageGuid::parse(&format!("msg-selected-{index:02}"))
            .map_err(|error| error.to_string())?,
        timestamp: MessageTimestamp::new(
            1_783_000_000 + i64::try_from(index).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        participant_count: 2,
        tapback_signal: false,
        excerpt: "Friday morning still good for the sync?".to_owned(),
        evidence_pointer: format!("messages://chat-a/msg-selected-{index:02}"),
    })
}
