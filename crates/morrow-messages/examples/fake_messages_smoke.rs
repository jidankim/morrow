use std::env;

use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionRequest, MessageGuid,
    MessageTimestamp, MessagesDataSource, MessagesError, NativeBatch, NativeReadRequest,
    ParticipantId, RawChat, RawMessage, TapbackKind, WhitelistedChat,
};

#[derive(Debug)]
struct Cli {
    permission: PermissionMode,
    whitelist: ChatGuid,
    backfill_days: BackfillDays,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionMode {
    Available,
    Denied,
}

#[derive(Debug, Clone)]
struct FakeMessages {
    batch: NativeBatch,
    permission: PermissionMode,
}

impl MessagesDataSource for FakeMessages {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        match self.permission {
            PermissionMode::Available => Ok(self.batch.clone()),
            PermissionMode::Denied => Err(MessagesError::PermissionDenied),
        }
    }
}

fn main() -> Result<(), MessagesError> {
    let cli = parse_cli()?;
    let now = MessageTimestamp::new(1_783_000_000)?;
    let request = request(cli.whitelist.clone(), cli.backfill_days, now)?;
    let denied_source = FakeMessages {
        batch: NativeBatch::default(),
        permission: cli.permission,
    };
    let denied_report = ingest_selected_threads(&denied_source, &request)?;
    println!("permission_status={:?}", denied_report.status);
    println!("permission_messages={}", denied_report.messages.len());
    println!("permission_warnings={}", denied_report.warnings.join(","));

    let selected_source = FakeMessages {
        batch: fixture_batch(cli.whitelist.as_str(), now.as_i64())?,
        permission: PermissionMode::Available,
    };
    let selected_report = ingest_selected_threads(&selected_source, &request)?;
    let selected_only = selected_report
        .messages
        .iter()
        .all(|message| message.chat_guid.as_str() == cli.whitelist.as_str());
    let non_whitelisted_like_present = selected_report
        .messages
        .iter()
        .any(|message| message.message_guid.as_str() == "msg-like");
    let old_message_present = selected_report
        .messages
        .iter()
        .any(|message| message.message_guid.as_str() == "msg-old");
    let swap_source = FakeMessages {
        batch: fixture_participant_swap(cli.whitelist.as_str(), now.as_i64())?,
        permission: PermissionMode::Available,
    };
    let swap_report = ingest_selected_threads(&swap_source, &request)?;
    let same_count_swap_paused = swap_report
        .paused_chats
        .iter()
        .any(|chat| chat.reason == "participant_identity_changed");
    println!("selected_messages={}", selected_report.messages.len());
    println!("selected_only={selected_only}");
    println!("non_whitelisted_like_present={non_whitelisted_like_present}");
    println!("old_backfill_message_present={old_message_present}");
    println!("same_count_swap_paused={same_count_swap_paused}");
    println!("PASS fake_messages_smoke");
    Ok(())
}

fn parse_cli() -> Result<Cli, MessagesError> {
    let mut args = env::args().skip(1);
    let mut permission = PermissionMode::Available;
    let mut whitelist = None;
    let mut backfill_days = None;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--permission" => {
                let value = next_arg(&mut args, "permission")?;
                permission = parse_permission(&value)?;
            }
            "--whitelist" => {
                let value = next_arg(&mut args, "whitelist")?;
                whitelist = Some(ChatGuid::parse(&value)?);
            }
            "--backfill-days" => {
                let value = next_arg(&mut args, "backfill_days")?;
                let parsed = value
                    .parse::<u8>()
                    .map_err(|_| MessagesError::InvalidInput {
                        field: "backfill_days",
                        reason: "must be an integer".to_owned(),
                    })?;
                backfill_days = Some(BackfillDays::new(parsed)?);
            }
            _ => {
                return Err(MessagesError::InvalidInput {
                    field: "args",
                    reason: "usage: fake_messages_smoke --permission <available|denied> --whitelist <chat-guid> --backfill-days <1-7>".to_owned(),
                });
            }
        }
    }

    Ok(Cli {
        permission,
        whitelist: whitelist.ok_or_else(|| MessagesError::InvalidInput {
            field: "whitelist",
            reason: "is required".to_owned(),
        })?,
        backfill_days: backfill_days.ok_or_else(|| MessagesError::InvalidInput {
            field: "backfill_days",
            reason: "is required".to_owned(),
        })?,
    })
}

fn next_arg(
    args: &mut impl Iterator<Item = String>,
    field: &'static str,
) -> Result<String, MessagesError> {
    args.next().ok_or_else(|| MessagesError::InvalidInput {
        field,
        reason: "missing value".to_owned(),
    })
}

fn parse_permission(value: &str) -> Result<PermissionMode, MessagesError> {
    match value {
        "available" => Ok(PermissionMode::Available),
        "denied" => Ok(PermissionMode::Denied),
        _ => Err(MessagesError::InvalidInput {
            field: "permission",
            reason: "must be available or denied".to_owned(),
        }),
    }
}

fn request(
    whitelist: ChatGuid,
    backfill_days: BackfillDays,
    now: MessageTimestamp,
) -> Result<IngestionRequest, MessagesError> {
    IngestionRequest::new(
        vec![WhitelistedChat::with_participants(
            whitelist,
            2,
            vec![
                participant("participant-hash-a")?,
                participant("participant-hash-b")?,
            ],
        )?],
        backfill_days,
        now,
    )
}

fn fixture_batch(selected_chat: &str, now: i64) -> Result<NativeBatch, MessagesError> {
    Ok(NativeBatch {
        chats: vec![
            RawChat {
                guid: ChatGuid::parse(selected_chat)?,
                participant_count: 2,
                participant_ids: vec![
                    participant("participant-hash-a")?,
                    participant("participant-hash-b")?,
                ],
                messages: vec![
                    raw_message(
                        selected_chat,
                        "msg-old",
                        now - 7 * 24 * 60 * 60 - 1,
                        "old appointment",
                        None,
                    )?,
                    raw_message(
                        selected_chat,
                        "msg-selected",
                        now - 60,
                        "appointment Friday at 2",
                        None,
                    )?,
                ],
            },
            RawChat {
                guid: ChatGuid::parse("chat-beta")?,
                participant_count: 2,
                participant_ids: Vec::new(),
                messages: vec![raw_message(
                    "chat-beta",
                    "msg-like",
                    now - 30,
                    "Liked: appointment Friday at 7",
                    Some(TapbackKind::Like),
                )?],
            },
        ],
    })
}

fn fixture_participant_swap(selected_chat: &str, now: i64) -> Result<NativeBatch, MessagesError> {
    Ok(NativeBatch {
        chats: vec![RawChat {
            guid: ChatGuid::parse(selected_chat)?,
            participant_count: 2,
            participant_ids: vec![
                participant("participant-hash-a")?,
                participant("participant-hash-c")?,
            ],
            messages: vec![raw_message(
                selected_chat,
                "msg-swapped",
                now - 45,
                "appointment Friday at 4",
                None,
            )?],
        }],
    })
}

fn raw_message(
    chat_guid: &str,
    message_guid: &str,
    timestamp: i64,
    text: &str,
    tapback: Option<TapbackKind>,
) -> Result<RawMessage, MessagesError> {
    Ok(RawMessage {
        chat_guid: ChatGuid::parse(chat_guid)?,
        message_guid: MessageGuid::parse(message_guid)?,
        timestamp: MessageTimestamp::new(timestamp)?,
        text: text.to_owned(),
        tapback,
    })
}

fn participant(value: &str) -> Result<ParticipantId, MessagesError> {
    ParticipantId::parse(value)
}
