use std::fmt;

use crate::sqlite_cli::row_value;
use crate::store::Store;
use crate::StorageError;

const SALT_BYTES: usize = 32;
const SALT_HEX_LEN: usize = SALT_BYTES * 2;
const SALT_JSON_LEN: usize = SALT_HEX_LEN + 2;

#[derive(Clone, PartialEq, Eq)]
pub struct ListIntakeSenderSalt {
    bytes: [u8; SALT_BYTES],
}

impl ListIntakeSenderSalt {
    pub const fn as_bytes(&self) -> &[u8; SALT_BYTES] {
        &self.bytes
    }
}

impl fmt::Debug for ListIntakeSenderSalt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ListIntakeSenderSalt(<local-private>)")
    }
}

impl Store {
    pub fn list_intake_sender_salt(&self) -> Result<ListIntakeSenderSalt, StorageError> {
        self.sqlite.execute(
            "INSERT OR IGNORE INTO settings (key, value_json, updated_at)
             VALUES (
                 'list_intake_sender_salt_v1',
                 '\"' || lower(hex(randomblob(32))) || '\"',
                 strftime('%s', 'now')
             );",
        )?;
        let rows = self.sqlite.query_rows(
            "SELECT value_json FROM settings WHERE key = 'list_intake_sender_salt_v1';",
        )?;
        let row = rows.first().ok_or_else(|| StorageError::Sqlite {
            message: "missing list intake sender salt".to_owned(),
        })?;
        parse_salt_json(row_value(row, 0, "settings.value_json")?)
    }
}

fn parse_salt_json(value: &str) -> Result<ListIntakeSenderSalt, StorageError> {
    if value.len() != SALT_JSON_LEN || !value.starts_with('"') || !value.ends_with('"') {
        return Err(invalid_salt());
    }
    let hex = &value[1..value.len() - 1];
    let mut bytes = [0_u8; SALT_BYTES];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0]).ok_or_else(invalid_salt)?;
        let low = hex_nibble(chunk[1]).ok_or_else(invalid_salt)?;
        bytes[index] = (high << 4) | low;
    }
    Ok(ListIntakeSenderSalt { bytes })
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn invalid_salt() -> StorageError {
    StorageError::InvalidInput {
        field: "list_intake_sender_salt",
        reason: "must be a local 32-byte hex JSON string".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_salt_json, SALT_BYTES};

    #[test]
    fn parse_salt_json_accepts_quoted_32_byte_hex() {
        let raw = "\"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"";

        let parsed = parse_salt_json(raw);
        assert!(parsed.is_ok(), "salt json parse failed");
        let Ok(salt) = parsed else {
            return;
        };

        assert_eq!(salt.as_bytes().len(), SALT_BYTES);
        assert_eq!(salt.as_bytes()[0], 0);
        assert_eq!(salt.as_bytes()[31], 31);
        assert!(!format!("{salt:?}").contains("000102"));
    }

    #[test]
    fn parse_salt_json_rejects_malformed_values() {
        for value in [
            "",
            "000102",
            "\"000102\"",
            "\"zz0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"",
        ] {
            assert!(parse_salt_json(value).is_err(), "{value}");
        }
    }
}
