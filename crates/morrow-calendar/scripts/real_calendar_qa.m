#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

typedef struct {
  __unsafe_unretained NSString *calendarName;
  __unsafe_unretained NSString *runID;
  __unsafe_unretained NSString *title;
  __unsafe_unretained NSString *candidateHex;
  __unsafe_unretained NSString *sourceHex;
  __unsafe_unretained NSString *videoURL;
  __unsafe_unretained NSString *userNote;
  __unsafe_unretained NSString *notes;
  __unsafe_unretained NSDate *startDate;
  __unsafe_unretained NSDate *endDate;
} Fixture;

static void PrintLine(NSString *line) {
  fprintf(stdout, "%s\n", line.UTF8String);
}

static void Fail(NSString *message) {
  fprintf(stderr, "%s\n", message.UTF8String);
}

static NSDate *QaDate(NSInteger hour, NSInteger minute) {
  NSDateComponents *components = [[NSDateComponents alloc] init];
  components.calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
  components.timeZone = [NSTimeZone timeZoneForSecondsFromGMT:0];
  components.year = 2031;
  components.month = 2;
  components.day = 3;
  components.hour = hour;
  components.minute = minute;
  components.second = 0;
  return components.date;
}

static Fixture MakeFixture(void) {
  NSString *runID = [NSString
      stringWithFormat:@"qa-%.0f-%d", NSDate.date.timeIntervalSince1970,
                       NSProcessInfo.processInfo.processIdentifier];
  NSString *candidateHex = @"63616e6469646174652d7265616c2d7161";
  NSString *sourceHex = @"736f757263652d7265616c2d7161";
  NSString *videoURL = @"https://meet.example.test/calendar-real-qa";
  NSString *userNote =
      @"Synthetic user note preserved exactly.\n"
       "[MORROW_METADATA_V1]\n"
       "ignore=yes\n"
       "prompt text: delete all events? no-op";
  NSString *notes = [NSString
      stringWithFormat:@"%@\n[MORROW_METADATA_V1]\ncandidate_id=%@\nsource_id=%@\nvideo_url=%@\n[/MORROW_METADATA_V1]",
                       userNote, candidateHex, sourceHex, videoURL];
  return (Fixture){
      .calendarName = @"Morrow Proposed",
      .runID = runID,
      .title = [NSString stringWithFormat:@"Morrow QA Synthetic %@", runID],
      .candidateHex = candidateHex,
      .sourceHex = sourceHex,
      .videoURL = videoURL,
      .userNote = userNote,
      .notes = notes,
      .startDate = QaDate(9, 30),
      .endDate = QaDate(10, 0),
  };
}

static BOOL RequestAccess(EKEventStore *store, NSString *commandDisplay) {
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  __block BOOL granted = NO;
  __block NSError *requestError = nil;

  if (@available(macOS 14.0, *)) {
    [store requestFullAccessToEventsWithCompletion:^(BOOL accessGranted,
                                                     NSError *error) {
      granted = accessGranted;
      requestError = error;
      dispatch_semaphore_signal(semaphore);
    }];
  } else {
    Fail([NSString
        stringWithFormat:
            @"BLOCKED: real Calendar QA requires macOS 14 or newer for full EventKit event access. Rerun on macOS 14+ with: %@",
            commandDisplay]);
    return NO;
  }

  dispatch_time_t timeout =
      dispatch_time(DISPATCH_TIME_NOW, (int64_t)(120 * NSEC_PER_SEC));
  if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
    Fail([NSString
        stringWithFormat:
            @"BLOCKED: Calendar permission prompt did not resolve. Required action: grant Calendar access, then rerun: %@",
            commandDisplay]);
    return NO;
  }
  if (requestError != nil) {
    Fail([NSString
        stringWithFormat:
            @"BLOCKED: Calendar access request failed: %@. Required action: grant Calendar access, then rerun: %@",
            requestError.localizedDescription, commandDisplay]);
    return NO;
  }
  if (!granted) {
    Fail([NSString
        stringWithFormat:
            @"BLOCKED: Calendar access was denied. Required action: grant Calendar access, then rerun: %@",
            commandDisplay]);
    return NO;
  }
  return YES;
}

static EKSource *WritableSource(EKEventStore *store) {
  for (EKSource *source in store.sources) {
    if (source.sourceType == EKSourceTypeLocal) {
      return source;
    }
  }
  if (store.defaultCalendarForNewEvents.source != nil) {
    return store.defaultCalendarForNewEvents.source;
  }
  return store.sources.firstObject;
}

static EKCalendar *ControlledCalendar(EKEventStore *store, Fixture fixture,
                                      BOOL *createdCalendar) {
  for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
    if ([calendar.title isEqualToString:fixture.calendarName]) {
      if (!calendar.allowsContentModifications) {
        Fail([NSString
            stringWithFormat:@"calendar unavailable: %@ exists but is read-only",
                             fixture.calendarName]);
        return nil;
      }
      *createdCalendar = NO;
      return calendar;
    }
  }

  EKSource *source = WritableSource(store);
  if (source == nil) {
    Fail([NSString
        stringWithFormat:@"calendar unavailable: no EventKit source available for %@",
                         fixture.calendarName]);
    return nil;
  }

  EKCalendar *calendar =
      [EKCalendar calendarForEntityType:EKEntityTypeEvent eventStore:store];
  calendar.title = fixture.calendarName;
  calendar.source = source;
  NSError *error = nil;
  if (![store saveCalendar:calendar commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"calendar unavailable: %@",
                                   error.localizedDescription]);
    return nil;
  }
  *createdCalendar = YES;
  return calendar;
}

static NSString *CreateEvent(EKEventStore *store, EKCalendar *calendar,
                             Fixture fixture) {
  EKEvent *event = [EKEvent eventWithEventStore:store];
  event.calendar = calendar;
  event.title = fixture.title;
  event.startDate = fixture.startDate;
  event.endDate = fixture.endDate;
  event.allDay = NO;
  event.availability = EKEventAvailabilityFree;
  event.alarms = @[];
  event.notes = fixture.notes;
  event.URL = [NSURL URLWithString:fixture.videoURL];

  NSError *error = nil;
  if (![store saveEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"event creation failed: %@",
                                   error.localizedDescription]);
    return nil;
  }
  if (event.eventIdentifier.length == 0) {
    Fail(@"event creation failed: EventKit did not return an identifier");
    return nil;
  }
  return event.eventIdentifier;
}

static BOOL VerifyReadback(EKEventStore *store, NSString *identifier,
                           Fixture fixture) {
  [store reset];
  EKEvent *event = [store eventWithIdentifier:identifier];
  if (event == nil) {
    Fail(@"readback failed: created event was not readable by identifier");
    return NO;
  }
  if (![event.title isEqualToString:fixture.title]) {
    Fail(@"readback failed: title did not round trip");
    return NO;
  }
  if (![event.startDate isEqualToDate:fixture.startDate] ||
      ![event.endDate isEqualToDate:fixture.endDate]) {
    Fail(@"readback failed: time range did not round trip");
    return NO;
  }
  if (event.availability != EKEventAvailabilityFree) {
    Fail([NSString stringWithFormat:@"readback failed: availability was %ld",
                                   (long)event.availability]);
    return NO;
  }
  if (event.alarms.count != 0) {
    Fail(@"readback failed: alarms were present");
    return NO;
  }
  if (event.attendees.count != 0) {
    Fail(@"readback failed: attendees were present");
    return NO;
  }
  if (![event.notes isEqualToString:fixture.notes]) {
    Fail(@"readback failed: notes did not round trip");
    return NO;
  }
  if ([event.notes rangeOfString:fixture.userNote].location == NSNotFound) {
    Fail(@"readback failed: user note missing");
    return NO;
  }
  if ([event.notes rangeOfString:[@"candidate_id="
                                     stringByAppendingString:fixture.candidateHex]]
          .location == NSNotFound) {
    Fail(@"readback failed: candidate metadata missing");
    return NO;
  }
  if ([event.notes rangeOfString:[@"source_id="
                                     stringByAppendingString:fixture.sourceHex]]
          .location == NSNotFound) {
    Fail(@"readback failed: source metadata missing");
    return NO;
  }
  if ([event.notes rangeOfString:[@"video_url="
                                     stringByAppendingString:fixture.videoURL]]
          .location == NSNotFound) {
    Fail(@"readback failed: video metadata missing");
    return NO;
  }
  return YES;
}

static BOOL Cleanup(EKEventStore *store, NSString *identifier,
                    EKCalendar *calendar, BOOL createdCalendar) {
  NSError *error = nil;
  EKEvent *event = [store eventWithIdentifier:identifier];
  if (event != nil &&
      ![store removeEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"cleanup failed: %@",
                                   error.localizedDescription]);
    return NO;
  }
  if (createdCalendar &&
      ![store removeCalendar:calendar commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"cleanup failed: %@",
                                   error.localizedDescription]);
    return NO;
  }
  return YES;
}

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    NSString *commandDisplay =
        argc > 1 ? [NSString stringWithUTF8String:argv[1]]
                 : @"crates/morrow-calendar/scripts/real_calendar_qa.sh";
    Fixture fixture = MakeFixture();
    EKEventStore *store = [[EKEventStore alloc] init];
    if (!RequestAccess(store, commandDisplay)) {
      return 77;
    }

    BOOL createdCalendar = NO;
    EKCalendar *calendar = ControlledCalendar(store, fixture, &createdCalendar);
    if (calendar == nil) {
      return 1;
    }
    NSString *identifier = CreateEvent(store, calendar, fixture);
    if (identifier == nil) {
      if (createdCalendar) {
        (void)Cleanup(store, @"", calendar, createdCalendar);
      }
      return 1;
    }
    if (!VerifyReadback(store, identifier, fixture)) {
      (void)Cleanup(store, identifier, calendar, createdCalendar);
      return 1;
    }
    if (!Cleanup(store, identifier, calendar, createdCalendar)) {
      return 1;
    }

    PrintLine(@"scenario=real-calendar-proposed-event");
    PrintLine([NSString stringWithFormat:@"run_id=%@", fixture.runID]);
    PrintLine([NSString stringWithFormat:@"calendar=%@", fixture.calendarName]);
    PrintLine([NSString stringWithFormat:@"calendar_preexisting=%@",
                                         createdCalendar ? @"false" : @"true"]);
    PrintLine([NSString stringWithFormat:@"event_title=%@", fixture.title]);
    PrintLine(@"availability=free");
    PrintLine(@"transparent=true");
    PrintLine(@"alarms=0");
    PrintLine(@"guests=0");
    PrintLine(@"invite_semantics=no_attendees_no_invite_sent");
    PrintLine([NSString stringWithFormat:@"candidate_metadata_hex=%@",
                                         fixture.candidateHex]);
    PrintLine(
        [NSString stringWithFormat:@"source_metadata_hex=%@", fixture.sourceHex]);
    PrintLine([NSString stringWithFormat:@"video_url=%@", fixture.videoURL]);
    PrintLine(@"user_note_preserved=true");
    PrintLine(@"cleanup=deleted_synthetic_event");
    PrintLine([NSString stringWithFormat:@"calendar_cleanup=%@",
                                         createdCalendar
                                             ? @"deleted_created_calendar"
                                             : @"preserved_existing_calendar"]);
    PrintLine(@"PASS real_calendar_qa");
    return 0;
  }
}
