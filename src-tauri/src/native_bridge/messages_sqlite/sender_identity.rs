use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
};

use morrow_messages::{ChatGuid, MessageGuid, MessageSenderIdentity, SenderKey};
use morrow_storage::ListIntakeSenderSalt;
use sha2::{Digest, Sha256};

const KEY_DOMAIN: &[u8] = b"morrow.list-intake.sender-key.v1:";
const KEY_PREFIX: &str = "senderKey-";
const DIGEST_BYTES: usize = 16;
const HEX: &[u8; 16] = b"0123456789abcdef";
const SENDER_SALT_DEBUG_MARKER: &str = "<redacted sender identity salt>";

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SenderIdentitySalt {
    bytes: [u8; 32],
}

impl SenderIdentitySalt {
    pub(super) fn from_local_store(salt: &ListIntakeSenderSalt) -> Self {
        Self {
            bytes: *salt.as_bytes(),
        }
    }

    pub(super) fn random_ephemeral() -> Result<Self, std::io::Error> {
        let mut bytes = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        Ok(Self { bytes })
    }

    pub(super) fn ephemeral_or_zero() -> Self {
        match Self::random_ephemeral() {
            Ok(salt) => salt,
            Err(_error) => Self { bytes: [0u8; 32] },
        }
    }
}

impl fmt::Debug for SenderIdentitySalt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SenderIdentitySalt")
            .field(&SENDER_SALT_DEBUG_MARKER)
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct SenderIdentityCache {
    inner: Arc<Mutex<SenderIdentityRows>>,
}

impl SenderIdentityCache {
    pub(super) fn replace(&self, rows: SenderIdentityRows) -> Result<(), &'static str> {
        let mut identities = self
            .inner
            .lock()
            .map_err(|_| "sender identity cache unavailable")?;
        *identities = rows;
        Ok(())
    }

    pub(super) fn get(
        &self,
        chat_guid: &ChatGuid,
        message_guid: &MessageGuid,
    ) -> MessageSenderIdentity {
        let Ok(identities) = self.inner.lock() else {
            return MessageSenderIdentity::unknown();
        };
        identities
            .get(&SenderIdentityCacheKey::new(chat_guid, message_guid))
            .cloned()
            .unwrap_or_else(MessageSenderIdentity::unknown)
    }
}

pub(super) type SenderIdentityRows = BTreeMap<SenderIdentityCacheKey, MessageSenderIdentity>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SenderIdentityCacheKey {
    chat_guid: String,
    message_guid: String,
}

impl SenderIdentityCacheKey {
    pub(super) fn new(chat_guid: &ChatGuid, message_guid: &MessageGuid) -> Self {
        Self {
            chat_guid: chat_guid.as_str().to_owned(),
            message_guid: message_guid.as_str().to_owned(),
        }
    }
}

pub(super) fn sender_identity(
    salt: &SenderIdentitySalt,
    chat_guid: &ChatGuid,
    raw_handle: &str,
) -> MessageSenderIdentity {
    let Some(normalized) = normalize_handle(raw_handle) else {
        return MessageSenderIdentity::unknown();
    };
    let mut hasher = Sha256::new();
    hasher.update(KEY_DOMAIN);
    hasher.update(salt.bytes);
    hasher.update(chat_guid.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    let key = format!("{}{}", KEY_PREFIX, hex_prefix(&digest));
    SenderKey::from_private_digest(&key).map_or_else(
        |_| MessageSenderIdentity::unknown(),
        MessageSenderIdentity::known,
    )
}

fn normalize_handle(raw_handle: &str) -> Option<String> {
    let trimmed = raw_handle.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

fn hex_prefix(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(DIGEST_BYTES * 2);
    for byte in bytes.iter().take(DIGEST_BYTES) {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{hex_prefix, SenderIdentitySalt, SENDER_SALT_DEBUG_MARKER};

    #[test]
    fn sender_identity_salt_debug_redacts_raw_salt_when_formatted() {
        // Given: a local sender salt contains private raw bytes.
        let raw = [0xab; 32];
        let salt = SenderIdentitySalt { bytes: raw };

        // When: debug formatting is requested.
        let debug = format!("{salt:?}");

        // Then: only the redaction marker is visible, not the bytes or hex digest material.
        assert!(debug.contains(SENDER_SALT_DEBUG_MARKER));
        assert!(!debug.contains("bytes"));
        assert!(!debug.contains(&format!("{raw:?}")));
        assert!(!debug.contains(&hex_prefix(&raw)));
    }
}
