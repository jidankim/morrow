use morrow_messages::{ChatGuid, MessagesError};

pub fn discovery_sql(limit: u16) -> String {
    format!(
        r#"
WITH ordered_participants AS (
  SELECT DISTINCT chj.chat_id, h.id AS handle_value, hex(h.id) AS handle_hex
  FROM chat_handle_join chj
  JOIN handle h ON h.ROWID = chj.handle_id
  ORDER BY chj.chat_id, h.id
),
participants AS (
  SELECT
    chat_id,
    COUNT(*) AS participant_count,
    group_concat(handle_hex, ',') AS participant_handles
  FROM ordered_participants
  GROUP BY chat_id
),
chat_meta AS (
  SELECT c.ROWID AS chat_rowid, c.guid, COALESCE(c.display_name, '') AS display_label, MAX(m.date) AS latest_date
  FROM chat c
  JOIN chat_message_join cmj ON cmj.chat_id = c.ROWID
  JOIN message m ON m.ROWID = cmj.message_id
  GROUP BY c.ROWID, c.guid, c.display_name
)
SELECT cm.guid, hex(cm.display_label), cm.latest_date, p.participant_count, p.participant_handles
FROM chat_meta cm
JOIN participants p ON p.chat_id = cm.chat_rowid
ORDER BY cm.latest_date DESC, cm.guid ASC
LIMIT {limit};
"#
    )
}

pub fn read_recent_sql(chat_guids: &[ChatGuid], limit: u16) -> Result<String, MessagesError> {
    let selected_values = chat_guids
        .iter()
        .map(|guid| Ok(format!("({})", sql_text(guid.as_str())?)))
        .collect::<Result<Vec<_>, MessagesError>>()?
        .join(",");
    Ok(format!(
        r#"
WITH selected(guid) AS (VALUES {selected_values}),
ordered_participants AS (
  SELECT DISTINCT chj.chat_id, h.id AS handle_value, hex(h.id) AS handle_hex
  FROM chat_handle_join chj
  JOIN handle h ON h.ROWID = chj.handle_id
  ORDER BY chj.chat_id, h.id
),
participants AS (
  SELECT
    chat_id,
    COUNT(*) AS participant_count,
    group_concat(handle_hex, ',') AS participant_handles
  FROM ordered_participants
  GROUP BY chat_id
),
selected_messages AS (
  SELECT
    c.guid AS chat_guid,
    p.participant_count,
    p.participant_handles,
    m.guid AS message_guid,
    m.date AS message_date,
    hex(COALESCE(m.text, '')) AS text_hex,
    hex(COALESCE(m.attributedBody, X'')) AS attributed_body_hex,
    ROW_NUMBER() OVER (PARTITION BY c.ROWID ORDER BY m.date DESC, m.ROWID DESC) AS row_number
  FROM selected s
  JOIN chat c ON c.guid = s.guid
  JOIN participants p ON p.chat_id = c.ROWID
  JOIN chat_message_join cmj ON cmj.chat_id = c.ROWID
  JOIN message m ON m.ROWID = cmj.message_id
)
SELECT chat_guid, participant_count, participant_handles, message_guid, message_date, text_hex, attributed_body_hex
FROM selected_messages
WHERE row_number <= {limit}
ORDER BY chat_guid ASC, message_date ASC, message_guid ASC;
"#
    ))
}

pub fn latest_previews_sql(chat_guids: &[ChatGuid]) -> Result<String, MessagesError> {
    let selected_values = chat_guids
        .iter()
        .map(|guid| Ok(format!("({})", sql_text(guid.as_str())?)))
        .collect::<Result<Vec<_>, MessagesError>>()?
        .join(",");
    Ok(format!(
        r#"
WITH selected(guid) AS (VALUES {selected_values}),
selected_messages AS (
  SELECT
    c.guid AS chat_guid,
    hex(COALESCE(m.text, '')) AS text_hex,
    hex(COALESCE(m.attributedBody, X'')) AS attributed_body_hex,
    ROW_NUMBER() OVER (PARTITION BY c.ROWID ORDER BY m.date DESC, m.ROWID DESC) AS row_number
  FROM selected s
  JOIN chat c ON c.guid = s.guid
  JOIN chat_message_join cmj ON cmj.chat_id = c.ROWID
  JOIN message m ON m.ROWID = cmj.message_id
)
SELECT chat_guid, text_hex, attributed_body_hex
FROM selected_messages
WHERE row_number = 1
ORDER BY chat_guid ASC;
"#
    ))
}

pub const fn all_chat_guids_sql() -> &'static str {
    "SELECT guid FROM chat ORDER BY guid ASC;"
}

fn sql_text(value: &str) -> Result<String, MessagesError> {
    if value.contains('\u{1f}') || value.contains('\0') {
        return Err(MessagesError::InvalidInput {
            field: "chat_guid",
            reason: "contains a reserved control character".to_owned(),
        });
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}
