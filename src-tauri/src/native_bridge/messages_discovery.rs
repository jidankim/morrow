use std::path::Path;

use morrow_messages::{MessagesDiscoveryDataSource, MessagesDiscoveryReport, MessagesError};

use super::messages_sqlite::MessagesSqliteAdapter;

pub(super) fn discover_chats_at(db_path: &Path) -> Result<MessagesDiscoveryReport, MessagesError> {
    MessagesSqliteAdapter::new(db_path.to_path_buf()).discover_chats()
}
