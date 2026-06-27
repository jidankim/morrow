use crate::CalendarError;

pub const PROPOSED_CALENDAR_NAME: &str = "Morrow Proposed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarSourceId(String);

impl CalendarSourceId {
    pub fn new(value: &str) -> Result<Self, CalendarError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CalendarError::InvalidInput {
                field: "source_id",
                reason: "source id must not be empty".to_owned(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateId(String);

impl CandidateId {
    pub fn new(value: &str) -> Result<Self, CalendarError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CalendarError::InvalidInput {
                field: "candidate_id",
                reason: "candidate id must not be empty".to_owned(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeRange {
    pub start_unix: i64,
    pub end_unix: i64,
}

impl TimeRange {
    pub fn new(start_unix: i64, end_unix: i64) -> Result<Self, CalendarError> {
        if end_unix <= start_unix {
            return Err(CalendarError::InvalidInput {
                field: "time_range",
                reason: "end must be after start".to_owned(),
            });
        }
        Ok(Self {
            start_unix,
            end_unix,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoUrl(String);

impl VideoUrl {
    pub fn new(value: &str) -> Result<Self, CalendarError> {
        let trimmed = value.trim();
        let has_supported_scheme =
            trimmed.starts_with("https://") || trimmed.starts_with("http://");
        let has_body_after_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .is_some_and(|rest| !rest.is_empty());
        let has_metadata_marker_chars = trimmed.contains('[') || trimmed.contains(']');
        let has_space_or_control = trimmed.chars().any(char::is_whitespace);
        if !has_supported_scheme
            || !has_body_after_scheme
            || has_metadata_marker_chars
            || has_space_or_control
        {
            return Err(CalendarError::InvalidInput {
                field: "video_url",
                reason: "video URL must use http or https without whitespace or note markers"
                    .to_owned(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Free,
    Busy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalMetadata {
    pub candidate_id: CandidateId,
    pub source_id: CalendarSourceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedEvent {
    pub title: String,
    pub time_range: TimeRange,
    pub user_note: String,
    pub video_url: Option<VideoUrl>,
    pub metadata: ProposalMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    pub calendar_name: String,
    pub calendar_source_id: CalendarSourceId,
    pub title: String,
    pub time_range: TimeRange,
    pub availability: Availability,
    pub alerts: Vec<String>,
    pub attendees: Vec<String>,
    pub invite_sent: bool,
    pub notes: String,
}
