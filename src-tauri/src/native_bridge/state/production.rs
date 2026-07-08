use std::path::Path;

use morrow_messages::{
    MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesDiscoveryStatus, MessagesError,
};

use super::SelectedChatIdResolutionCache;
use crate::native_bridge::{
    messages_sqlite, MessagesPreviewCommandReport, MessagesPreviewRequest, MorrowTokenVault,
};

#[derive(Debug, Default)]
pub(super) struct ProductionNativeBridge {
    pub(super) token_vault: MorrowTokenVault,
    pub(super) selected_chat_id_cache: SelectedChatIdResolutionCache,
}

impl ProductionNativeBridge {
    pub(super) fn discover_messages_chats_at(
        &self,
        db_path: &Path,
    ) -> Result<MessagesDiscoveryReport, MessagesError> {
        let report =
            messages_sqlite::MessagesSqliteAdapter::new(db_path.to_path_buf()).discover_chats()?;
        if report.status() == MessagesDiscoveryStatus::Ready {
            self.selected_chat_id_cache
                .store_ready_report(db_path, &report)?;
        }
        Ok(report)
    }

    pub(super) fn load_messages_chat_previews_at(
        &self,
        db_path: &Path,
        request: &MessagesPreviewRequest,
    ) -> Result<MessagesPreviewCommandReport, MessagesError> {
        let source = messages_sqlite::MessagesSqliteAdapter::new(db_path.to_path_buf());
        let chat_guids = match self
            .selected_chat_id_cache
            .resolve(db_path, &request.chat_ids)?
        {
            Some(chat_guids) => chat_guids,
            None => source.resolve_public_chat_ids(&request.chat_ids)?,
        };
        source.load_messages_chat_previews_for_chat_guids(&chat_guids)
    }
}
