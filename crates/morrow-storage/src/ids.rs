use crate::types::CandidateKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(String);

impl CandidateId {
    pub fn derive(
        kind: CandidateKind,
        chat_guid: &str,
        anchor_message_guid: &str,
        normalized_time: &str,
    ) -> Self {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for part in [
            kind.as_str(),
            chat_guid,
            anchor_message_guid,
            normalized_time,
        ] {
            for byte in part.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(format!("morrow_{hash:016x}"))
    }

    pub fn from_storage(raw: &str) -> Result<Self, crate::StorageError> {
        let hex = raw
            .strip_prefix("morrow_")
            .ok_or_else(|| crate::StorageError::InvalidInput {
                field: "candidate_id",
                reason: "missing morrow_ prefix".to_owned(),
            })?;
        let is_valid = hex.len() == 16 && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
        if is_valid {
            Ok(Self(raw.to_owned()))
        } else {
            Err(crate::StorageError::InvalidInput {
                field: "candidate_id",
                reason: "expected 16 lowercase hex characters".to_owned(),
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CandidateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
