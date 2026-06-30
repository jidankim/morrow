/// Evidence explaining why a mapped proposal disappeared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisappearanceEvidence {
    /// The external adapter can prove the user deleted the proposed item.
    UserDeletedProposed,
    /// The adapter lost visibility or the fake store was reset.
    StoreResetOrPermissionGap,
    /// No Morrow metadata or external mapping could be observed.
    NoMorrowMetadata,
}

/// A typed observation from Calendar, Reminders, or a fake external store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalItemObservation {
    /// The object is still in the Morrow proposed surface.
    Pending,
    /// The object is still proposed, but the user edited it.
    PendingEdited {
        /// The externally observed title after user edits.
        observed_title: Option<String>,
        /// The externally observed normalized time after user edits.
        observed_normalized_time: Option<String>,
    },
    /// The proposed object was moved out of the proposed surface.
    ApprovedByMove {
        /// The real external object ID.
        external_object_id: String,
        /// The real external source/list/calendar ID.
        external_source_id: String,
    },
    /// A matching real object was copied out while the proposed item remains.
    ApprovedByCopy {
        /// The real external object ID.
        external_object_id: String,
        /// The real external source/list/calendar ID.
        external_source_id: String,
    },
    /// The proposed item was deleted.
    DeletedFromProposed,
    /// The mapped item disappeared.
    Disappeared {
        /// Evidence controlling unknown-vs-rejected handling.
        evidence: DisappearanceEvidence,
    },
    /// The observed approved item is completed.
    Completed,
    /// External creation failed before a durable item could be observed.
    CreationFailed,
}

impl ExternalItemObservation {
    /// Builds a move-approval observation.
    pub fn approved_by_move(external_object_id: &str, external_source_id: &str) -> Self {
        Self::ApprovedByMove {
            external_object_id: external_object_id.to_owned(),
            external_source_id: external_source_id.to_owned(),
        }
    }

    /// Builds a copy-approval observation.
    pub fn approved_by_copy(external_object_id: &str, external_source_id: &str) -> Self {
        Self::ApprovedByCopy {
            external_object_id: external_object_id.to_owned(),
            external_source_id: external_source_id.to_owned(),
        }
    }
}
