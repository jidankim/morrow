mod diagnostic;
mod row;
mod seen;
mod sender_label;
mod sql;

use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{
    ListIntakeAggregateQuery, ListIntakeAggregateRow, ListIntakeExtractionDraft,
    ListIntakeItemDraft, ListIntakeProposal, ListIntakeProposalDecision, ListIntakeProposalStatus,
    ListIntakeReviewProposal, ListIntakeStorageReceipt,
};
use crate::StorageError;

use super::Store;
use row::{
    extraction_item_key, extraction_proposal_key, parse_item_row, parse_proposal_row,
    proposal_entry_key, storage_id, validate_extraction, validate_item_with_categories,
};
use sql::{
    aggregate_row, aggregate_sql, count_delta, entry_insert_sql, proposal_insert_sql,
    proposal_item_insert_sql, proposal_source_draft,
};

impl Store {
    pub fn record_list_intake_extraction(
        &self,
        draft: ListIntakeExtractionDraft,
    ) -> Result<ListIntakeStorageReceipt, StorageError> {
        validate_extraction(&draft)?;
        if self.list_intake_extraction_seen(&draft)? {
            return Ok(ListIntakeStorageReceipt {
                approved_entry_count: 0,
                proposal_count: 0,
            });
        }
        let entries_before = self.list_intake_table_count("list_intake_entries")?;
        let proposals_before = self.list_intake_table_count("list_intake_proposals")?;
        let mut sql = String::from("BEGIN;\n");
        match draft.confidence_tier {
            crate::types::ListIntakeConfidenceTier::Low => {}
            crate::types::ListIntakeConfidenceTier::Review => {
                let proposal_key = extraction_proposal_key(&draft);
                let proposal_id = storage_id("list_intake_proposal", &proposal_key);
                sql.push_str(&proposal_insert_sql(&draft, &proposal_key)?);
                for (item_index, item) in draft.items.iter().enumerate() {
                    let key = extraction_item_key(&draft, item_index);
                    sql.push_str(&proposal_item_insert_sql(
                        &proposal_id,
                        item,
                        item_index,
                        &key,
                        draft.observed_at,
                    )?);
                }
            }
            crate::types::ListIntakeConfidenceTier::AutoAggregate => {
                for (item_index, item) in draft.items.iter().enumerate() {
                    let key = extraction_item_key(&draft, item_index);
                    sql.push_str(&entry_insert_sql(&draft, item, item_index, &key, None)?);
                }
            }
        }
        sql.push_str("COMMIT;");
        self.sqlite.execute(&sql)?;
        Ok(ListIntakeStorageReceipt {
            approved_entry_count: count_delta(
                entries_before,
                self.list_intake_table_count("list_intake_entries")?,
            )?,
            proposal_count: count_delta(
                proposals_before,
                self.list_intake_table_count("list_intake_proposals")?,
            )?,
        })
    }

    pub fn list_intake_proposals(
        &self,
        profile_id: &str,
    ) -> Result<Vec<ListIntakeProposal>, StorageError> {
        crate::validation::validate_text("profile_id", profile_id, 80)?;
        let sql = format!(
            "SELECT proposal_id, status, profile_id, message_guid, sender_label,
                    created_at
             FROM list_intake_proposals
             WHERE profile_id = {}
             ORDER BY created_at, proposal_id;",
            sql_text(profile_id)?
        );
        let mut proposals = Vec::new();
        for row in self.sqlite.query_rows(&sql)? {
            let proposal_id = row_value(&row, 0, "proposal_id")?;
            proposals.push(parse_proposal_row(
                &row,
                self.proposal_item_rows(proposal_id)?
                    .into_iter()
                    .map(|item_row| parse_item_row(&item_row))
                    .collect::<Result<Vec<_>, _>>()?,
            )?);
        }
        Ok(proposals)
    }

    pub fn list_intake_review_proposals(
        &self,
    ) -> Result<Vec<ListIntakeReviewProposal>, StorageError> {
        let sql = "SELECT proposal_id, profile_id, chat_key, sender_label
                   FROM list_intake_proposals
                   WHERE status = 'pending'
                   ORDER BY created_at, proposal_id;";
        self.sqlite
            .query_rows(sql)?
            .into_iter()
            .map(|row| {
                let proposal_id = row_value(&row, 0, "proposal_id")?.to_owned();
                Ok(ListIntakeReviewProposal {
                    proposal_id: proposal_id.clone(),
                    profile_id: row_value(&row, 1, "profile_id")?.to_owned(),
                    chat_key: row_value(&row, 2, "chat_key")?.to_owned(),
                    sender_label: row_value(&row, 3, "sender_label")?.to_owned(),
                    items: self
                        .proposal_item_rows(&proposal_id)?
                        .into_iter()
                        .map(|item_row| parse_item_row(&item_row))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect()
    }

    pub fn decide_list_intake_proposal(
        &self,
        decision: ListIntakeProposalDecision,
    ) -> Result<(), StorageError> {
        match decision {
            ListIntakeProposalDecision::ApproveEdited {
                proposal_id,
                items,
                allowed_category_ids,
                decided_at,
            } => self.approve_list_intake_proposal(
                &proposal_id,
                items,
                &allowed_category_ids,
                decided_at,
            ),
            ListIntakeProposalDecision::Reject {
                proposal_id,
                decided_at,
            } => self.reject_list_intake_proposal(&proposal_id, decided_at),
        }
    }

    pub fn list_intake_aggregates(
        &self,
        query: ListIntakeAggregateQuery,
    ) -> Result<Vec<ListIntakeAggregateRow>, StorageError> {
        let sql = aggregate_sql(&query)?;
        self.sqlite
            .query_rows(&sql)?
            .into_iter()
            .map(|row| aggregate_row(&row))
            .collect()
    }

    fn approve_list_intake_proposal(
        &self,
        proposal_id: &str,
        items: Vec<ListIntakeItemDraft>,
        allowed_category_ids: &[String],
        decided_at: i64,
    ) -> Result<(), StorageError> {
        crate::validation::validate_text("proposal_id", proposal_id, 80)?;
        if items.is_empty() {
            return Err(StorageError::InvalidInput {
                field: "items",
                reason: "must contain at least one item".to_owned(),
            });
        }
        for item in &items {
            validate_item_with_categories(item, allowed_category_ids)?;
        }
        let rows = self.proposal_source_rows(proposal_id)?;
        let row = rows.first().ok_or_else(|| StorageError::InvalidInput {
            field: "proposal_id",
            reason: "proposal not found".to_owned(),
        })?;
        if ListIntakeProposalStatus::parse(row_value(row, 0, "status")?)?
            != ListIntakeProposalStatus::Pending
        {
            return Ok(());
        }
        let draft = proposal_source_draft(row, decided_at)?;
        let mut sql = String::from("BEGIN;\n");
        for (item_index, item) in items.iter().enumerate() {
            let key = proposal_entry_key(proposal_id, item_index);
            sql.push_str(&entry_insert_sql(
                &draft,
                item,
                item_index,
                &key,
                Some(proposal_id),
            )?);
        }
        sql.push_str(&format!(
            "UPDATE list_intake_proposals
             SET status = 'approved', decided_at = {decided_at}, updated_at = {decided_at}
             WHERE proposal_id = {} AND status = 'pending';\n",
            sql_text(proposal_id)?
        ));
        sql.push_str("COMMIT;");
        self.sqlite.execute(&sql)
    }

    fn reject_list_intake_proposal(
        &self,
        proposal_id: &str,
        decided_at: i64,
    ) -> Result<(), StorageError> {
        crate::validation::validate_text("proposal_id", proposal_id, 80)?;
        let sql = format!(
            "UPDATE list_intake_proposals
             SET status = 'rejected', decided_at = {decided_at}, updated_at = {decided_at}
             WHERE proposal_id = {} AND status = 'pending';",
            sql_text(proposal_id)?
        );
        self.sqlite.execute(&sql)
    }

    fn proposal_source_rows(&self, proposal_id: &str) -> Result<Vec<Vec<String>>, StorageError> {
        let sql = format!(
            "SELECT status, profile_id, profile_version, examples_hash, message_guid, evidence_pointer,
                    chat_key, sender_key, window_local_date, window_timezone,
                    window_start_unix_seconds, confidence_millis, confidence_tier
             FROM list_intake_proposals
             WHERE proposal_id = {};",
            sql_text(proposal_id)?
        );
        self.sqlite.query_rows(&sql)
    }

    fn proposal_item_rows(&self, proposal_id: &str) -> Result<Vec<Vec<String>>, StorageError> {
        let sql = format!(
            "SELECT source_item_index, quantity, item_name, unit, category_id
             FROM list_intake_proposal_items
             WHERE proposal_id = {}
             ORDER BY source_item_index;",
            sql_text(proposal_id)?
        );
        self.sqlite.query_rows(&sql)
    }
}
