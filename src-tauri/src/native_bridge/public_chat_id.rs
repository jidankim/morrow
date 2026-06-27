use morrow_messages::ChatGuid;
use sha2::{Digest, Sha256};

const CHAT_DOMAIN: &[u8] = b"morrow.messages.public-chat-id.v1:";
const PARTICIPANT_DOMAIN: &[u8] = b"morrow.messages.public-participant-id.v1:";
const MESSAGE_DOMAIN: &[u8] = b"morrow.messages.public-message-id.v1:";
const CHAT_PREFIX: &str = "messages-chat-";
const PARTICIPANT_PREFIX: &str = "messages-participant-";
const MESSAGE_PREFIX: &str = "messages-message-";
const HEX: &[u8; 16] = b"0123456789abcdef";
const DIGEST_BYTES: usize = 16;

pub fn public_chat_id(chat_guid: &ChatGuid) -> String {
    public_id(CHAT_DOMAIN, CHAT_PREFIX, chat_guid.as_str())
}

pub fn public_participant_id(raw_handle: &str) -> String {
    public_id(PARTICIPANT_DOMAIN, PARTICIPANT_PREFIX, raw_handle)
}

pub fn public_message_id(raw_message_guid: &str) -> String {
    public_id(MESSAGE_DOMAIN, MESSAGE_PREFIX, raw_message_guid)
}

fn public_id(domain: &[u8], prefix: &str, raw_value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(raw_value.as_bytes());
    let digest = hasher.finalize();
    let mut value = String::with_capacity(prefix.len() + DIGEST_BYTES * 2);
    value.push_str(prefix);
    for byte in digest.iter().take(DIGEST_BYTES) {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}
