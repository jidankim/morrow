mod adapter;
mod error;
mod fake;
mod metadata;
mod model;

pub use adapter::{CalendarAdapter, CalendarPlanner};
pub use error::CalendarError;
pub use fake::FakeEventKit;
pub use metadata::{format_notes, parse_metadata, strip_morrow_metadata};
pub use model::{
    Availability, CalendarSourceId, CandidateId, EventRecord, ProposalMetadata, ProposedEvent,
    TimeRange, VideoUrl, PROPOSED_CALENDAR_NAME,
};
