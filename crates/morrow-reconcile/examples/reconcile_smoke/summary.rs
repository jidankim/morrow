use std::io::{Error, Write};

const EXPECTED_LIFECYCLE_READBACK_COUNT: usize = 1;

#[derive(Debug, Default)]
/// Lifecycle smoke readback counts.
pub struct SmokeCounts {
    /// Approved-by-move readback count.
    pub approved_by_move: usize,
    /// Approved-by-copy cleanup readback count.
    pub approved_by_copy_cleanup: usize,
    /// Rejected-by-delete readback count.
    pub rejected_by_delete: usize,
    /// Unknown-disappearance readback count.
    pub unknown_disappearance: usize,
    /// Edited-pending suppression readback count.
    pub edited_pending_no_overwrite: usize,
    /// Completed-closure readback count.
    pub completed_closure: usize,
    /// Partial-write recovery readback count.
    pub partial_write_recovered: usize,
}

impl SmokeCounts {
    const fn required_readbacks(&self) -> [(&'static str, usize); 7] {
        [
            ("approved_by_move", self.approved_by_move),
            ("approved_by_copy_cleanup", self.approved_by_copy_cleanup),
            ("rejected_by_delete", self.rejected_by_delete),
            ("unknown_disappearance", self.unknown_disappearance),
            (
                "edited_pending_no_overwrite",
                self.edited_pending_no_overwrite,
            ),
            ("completed_closure", self.completed_closure),
            ("partial_write_recovered", self.partial_write_recovered),
        ]
    }

    fn validate_required_readbacks(&self) -> Result<(), Error> {
        for (name, observed) in self.required_readbacks() {
            if observed != EXPECTED_LIFECYCLE_READBACK_COUNT {
                return Err(Error::other(format!(
                    "{name} expected {EXPECTED_LIFECYCLE_READBACK_COUNT}, observed {observed}"
                )));
            }
        }
        Ok(())
    }
}

/// Writes and validates lifecycle smoke readback counts.
pub fn write_summary(counts: &SmokeCounts) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout();
    for (name, observed) in counts.required_readbacks() {
        writeln!(out, "{name}={observed}")?;
    }
    counts.validate_required_readbacks()?;
    writeln!(out, "PASS reconcile_smoke")?;
    Ok(())
}
