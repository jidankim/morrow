use crate::sqlite_cli::sql_text;
use crate::validation::validate_text;
use crate::StorageError;

use super::Store;

impl Store {
    pub fn delete_provider_route_outcome(
        &self,
        route_fingerprint: &str,
    ) -> Result<(), StorageError> {
        validate_text("route_fingerprint", route_fingerprint, 160)?;
        let sql = format!(
            "DELETE FROM provider_route_outcomes WHERE route_fingerprint = {};",
            sql_text(route_fingerprint)?
        );
        self.sqlite.execute(&sql)
    }
}
