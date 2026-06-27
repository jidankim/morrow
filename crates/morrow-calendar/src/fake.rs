use crate::{
    CalendarAdapter, CalendarError, CalendarSourceId, EventRecord, PROPOSED_CALENDAR_NAME,
};

#[derive(Debug, Default)]
pub struct FakeEventKit {
    pub calendars: Vec<String>,
    pub events: Vec<EventRecord>,
    pub selected_sources: Vec<CalendarSourceId>,
    fail_calendar_creation: Option<String>,
    fail_event_creation: Option<String>,
}

impl FakeEventKit {
    pub fn with_existing_proposed_calendar() -> Self {
        Self {
            calendars: vec![PROPOSED_CALENDAR_NAME.to_owned()],
            events: Vec::new(),
            selected_sources: Vec::new(),
            fail_calendar_creation: None,
            fail_event_creation: None,
        }
    }

    pub fn fail_calendar_creation(reason: &str) -> Self {
        Self {
            calendars: Vec::new(),
            events: Vec::new(),
            selected_sources: Vec::new(),
            fail_calendar_creation: Some(reason.to_owned()),
            fail_event_creation: None,
        }
    }

    pub fn fail_event_creation_after_calendar(reason: &str) -> Self {
        Self {
            calendars: Vec::new(),
            events: Vec::new(),
            selected_sources: Vec::new(),
            fail_calendar_creation: None,
            fail_event_creation: Some(reason.to_owned()),
        }
    }

    pub fn recover_event_creation(mut self) -> Self {
        self.fail_event_creation = None;
        self
    }
}

impl CalendarAdapter for FakeEventKit {
    fn ensure_calendar(&mut self, source_id: &CalendarSourceId) -> Result<String, CalendarError> {
        self.selected_sources.push(source_id.clone());
        if let Some(reason) = &self.fail_calendar_creation {
            return Err(CalendarError::CalendarCreationFailed {
                source_id: source_id.as_str().to_owned(),
                reason: reason.clone(),
            });
        }
        if !self
            .calendars
            .iter()
            .any(|calendar| calendar == PROPOSED_CALENDAR_NAME)
        {
            self.calendars.push(PROPOSED_CALENDAR_NAME.to_owned());
        }
        Ok(PROPOSED_CALENDAR_NAME.to_owned())
    }

    fn create_event(&mut self, event: EventRecord) -> Result<String, CalendarError> {
        if let Some(reason) = &self.fail_event_creation {
            return Err(CalendarError::EventCreationFailed {
                reason: reason.clone(),
            });
        }
        self.events.push(event);
        Ok(format!("fake-event-{}", self.events.len()))
    }
}
