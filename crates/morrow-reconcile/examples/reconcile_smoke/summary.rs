#![allow(clippy::redundant_pub_crate)]

use std::io::{Error, Write};

const EXPECTED_LIFECYCLE_READBACK_COUNT: usize = 1;

#[derive(Debug, Default)]
pub(crate) struct SmokeCounts {
    pub(crate) approved_by_move: usize,
    pub(crate) approved_by_copy_cleanup: usize,
    pub(crate) rejected_by_delete: usize,
    pub(crate) unknown_disappearance: usize,
    pub(crate) edited_pending_no_overwrite: usize,
    pub(crate) completed_closure: usize,
    pub(crate) partial_write_recovered: usize,
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

pub(crate) fn write_summary(counts: &SmokeCounts) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout();
    for (name, observed) in counts.required_readbacks() {
        writeln!(out, "{name}={observed}")?;
    }
    counts.validate_required_readbacks()?;
    writeln!(out, "PASS reconcile_smoke")?;
    Ok(())
}
