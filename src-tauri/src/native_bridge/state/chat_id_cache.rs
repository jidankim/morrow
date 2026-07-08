use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};

use morrow_messages::{ChatGuid, MessagesDiscoveryReport, MessagesError};

use crate::native_bridge::public_chat_id::public_chat_id;

#[derive(Debug, Default)]
pub(in crate::native_bridge) struct SelectedChatIdResolutionCache {
    entry: Mutex<Option<SelectedChatIdResolutionCacheEntry>>,
}

#[derive(Debug)]
struct SelectedChatIdResolutionCacheEntry {
    db_path: PathBuf,
    db_fingerprint: DatabaseFingerprint,
    chat_guid_by_public_id: BTreeMap<String, ChatGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DatabaseFingerprint {
    len: u64,
    modified: SystemTime,
}

impl SelectedChatIdResolutionCache {
    pub(super) fn store_ready_report(
        &self,
        db_path: &Path,
        report: &MessagesDiscoveryReport,
    ) -> Result<(), MessagesError> {
        let chat_guid_by_public_id = report
            .chats()
            .iter()
            .map(|chat| (public_chat_id(chat.chat_guid()), chat.chat_guid().clone()))
            .collect();
        let mut entry = self.entry.lock().map_err(|_| cache_unavailable())?;
        let Some(db_fingerprint) = database_fingerprint(db_path) else {
            *entry = None;
            return Ok(());
        };
        *entry = Some(SelectedChatIdResolutionCacheEntry {
            db_path: db_path.to_path_buf(),
            db_fingerprint,
            chat_guid_by_public_id,
        });
        Ok(())
    }

    pub(in crate::native_bridge) fn resolve(
        &self,
        db_path: &Path,
        public_ids: &[String],
    ) -> Result<Option<Vec<ChatGuid>>, MessagesError> {
        let Some(db_fingerprint) = database_fingerprint(db_path) else {
            return Ok(None);
        };
        let entry = self.entry.lock().map_err(|_| cache_unavailable())?;
        let Some(entry) = entry
            .as_ref()
            .filter(|entry| entry.db_path == db_path && entry.db_fingerprint == db_fingerprint)
        else {
            return Ok(None);
        };
        Ok(public_ids
            .iter()
            .map(|public_id| entry.chat_guid_by_public_id.get(public_id).cloned())
            .collect::<Option<Vec<_>>>())
    }
}

fn database_fingerprint(db_path: &Path) -> Option<DatabaseFingerprint> {
    let metadata = fs::metadata(db_path).ok()?;
    let modified = metadata.modified().ok()?;
    Some(DatabaseFingerprint {
        len: metadata.len(),
        modified,
    })
}

fn cache_unavailable() -> MessagesError {
    MessagesError::NativeUnavailable {
        reason: "native selected chat id cache is unavailable".to_owned(),
    }
}
