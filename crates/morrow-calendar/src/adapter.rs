use crate::{
    format_notes, Availability, CalendarError, CalendarSourceId, EventRecord, ProposedEvent,
};

pub trait CalendarAdapter {
    fn ensure_calendar(&mut self, source_id: &CalendarSourceId) -> Result<String, CalendarError>;
    fn create_event(&mut self, event: EventRecord) -> Result<String, CalendarError>;
}

#[derive(Debug)]
pub struct CalendarPlanner<A> {
    adapter: A,
}

impl<A: CalendarAdapter> CalendarPlanner<A> {
    pub const fn new(adapter: A) -> Self {
        Self { adapter }
    }

    pub fn propose_event(&mut self, event: ProposedEvent) -> Result<String, CalendarError> {
        let ProposedEvent {
            title,
            time_range,
            user_note,
            video_url,
            metadata,
        } = event;
        let calendar_name = self.adapter.ensure_calendar(&metadata.source_id)?;
        let notes = format_notes(&user_note, video_url.as_ref(), &metadata);
        let record = EventRecord {
            calendar_name,
            calendar_source_id: metadata.source_id,
            title,
            time_range,
            availability: Availability::Free,
            alerts: Vec::new(),
            attendees: Vec::new(),
            invite_sent: false,
            notes,
        };
        self.adapter.create_event(record)
    }

    pub fn into_adapter(self) -> A {
        self.adapter
    }
}
