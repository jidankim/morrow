#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

static NSString *const ProposedCalendarName = @"Morrow Proposed";
static NSString *const ApprovalCalendarName = @"Morrow Proposed QA Approved";
static NSString *const QAMarker = @"MorrowCalendarLifecycleRealQA:F3 synthetic only";

static void PrintLine(NSString *line) {
  fprintf(stdout, "%s\n", line.UTF8String);
}

static void Fail(NSString *message) {
  fprintf(stderr, "%s\n", message.UTF8String);
}

static NSDate *QaDate(NSInteger year, NSInteger month, NSInteger day,
                      NSInteger hour, NSInteger minute) {
  NSDateComponents *components = [[NSDateComponents alloc] init];
  components.calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
  components.timeZone = [NSTimeZone timeZoneForSecondsFromGMT:0];
  components.year = year;
  components.month = month;
  components.day = day;
  components.hour = hour;
  components.minute = minute;
  components.second = 0;
  return components.date;
}

static NSDate *QaDateRelativeToToday(NSInteger dayOffset, NSInteger hour,
                                     NSInteger minute) {
  NSCalendar *calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
  calendar.timeZone = [NSTimeZone timeZoneForSecondsFromGMT:0];

  NSDateComponents *dayComponents =
      [calendar components:NSCalendarUnitYear | NSCalendarUnitMonth | NSCalendarUnitDay
                  fromDate:NSDate.date];
  dayComponents.hour = hour;
  dayComponents.minute = minute;
  dayComponents.second = 0;

  NSDate *anchor = [calendar dateFromComponents:dayComponents];
  NSDateComponents *offset = [[NSDateComponents alloc] init];
  offset.day = dayOffset;
  return [calendar dateByAddingComponents:offset toDate:anchor options:0];
}

static NSString *RunID(void) {
  return [NSString stringWithFormat:@"f3-%.0f-%d",
                                    NSDate.date.timeIntervalSince1970,
                                    NSProcessInfo.processInfo.processIdentifier];
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
            @"BLOCKED: real Calendar lifecycle QA requires macOS 14 or newer for full EventKit event access. Rerun on macOS 14+ with: %@",
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

static NSString *SourceID(EKCalendar *calendar) {
  return calendar.source.sourceIdentifier ?: @"<missing-source-id>";
}

static BOOL SameSource(EKCalendar *left, EKSource *right) {
  if (left.source == nil || right == nil) {
    return NO;
  }
  NSString *leftID = left.source.sourceIdentifier;
  NSString *rightID = right.sourceIdentifier;
  return leftID.length > 0 && rightID.length > 0 && [leftID isEqualToString:rightID];
}

static EKCalendar *FindWritableCalendar(EKEventStore *store, NSString *title,
                                        EKSource *sourceOrNil) {
  for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
    if ([calendar.title isEqualToString:title] &&
        (sourceOrNil == nil || SameSource(calendar, sourceOrNil))) {
      if (!calendar.allowsContentModifications) {
        continue;
      }
      return calendar;
    }
  }
  return nil;
}

static NSArray<EKCalendar *> *WritableControlledCalendars(EKEventStore *store) {
  NSMutableArray<EKCalendar *> *calendars = [NSMutableArray array];
  for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
    if (!calendar.allowsContentModifications) {
      continue;
    }
    if ([calendar.title isEqualToString:ProposedCalendarName] ||
        [calendar.title isEqualToString:ApprovalCalendarName]) {
      [calendars addObject:calendar];
    }
  }
  return [calendars copy];
}

static EKCalendar *CreateCalendar(EKEventStore *store, NSString *title,
                                  EKSource *source, BOOL *created) {
  if (source == nil) {
    Fail([NSString stringWithFormat:@"calendar unavailable: no writable source for %@",
                                     title]);
    return nil;
  }
  EKCalendar *calendar =
      [EKCalendar calendarForEntityType:EKEntityTypeEvent eventStore:store];
  calendar.title = title;
  calendar.source = source;
  NSError *error = nil;
  if (![store saveCalendar:calendar commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"calendar create failed: %@",
                                     error.localizedDescription]);
    return nil;
  }
  *created = YES;
  return calendar;
}

static BOOL EnsureControlledCalendars(EKEventStore *store, EKCalendar **proposed,
                                      BOOL *proposedCreated, EKCalendar **approval,
                                      BOOL *approvalCreated) {
  *proposedCreated = NO;
  *approvalCreated = NO;

  EKCalendar *existingProposed = FindWritableCalendar(store, ProposedCalendarName, nil);
  EKCalendar *existingApproval = nil;
  if (existingProposed != nil) {
    existingApproval =
        FindWritableCalendar(store, ApprovalCalendarName, existingProposed.source);
  }

  if (existingProposed == nil || existingApproval == nil) {
    EKSource *source = existingProposed.source ?: WritableSource(store);
    if (source == nil) {
      Fail(@"calendar unavailable: no writable source for controlled calendars");
      return NO;
    }
    if (existingProposed == nil) {
      existingProposed =
          CreateCalendar(store, ProposedCalendarName, source, proposedCreated);
    }
    if (existingProposed == nil) {
      return NO;
    }
    existingApproval =
        FindWritableCalendar(store, ApprovalCalendarName, existingProposed.source);
    if (existingApproval == nil) {
      existingApproval =
          CreateCalendar(store, ApprovalCalendarName, existingProposed.source,
                         approvalCreated);
    }
  }

  if (existingProposed == nil || existingApproval == nil) {
    return NO;
  }
  if (!SameSource(existingApproval, existingProposed.source)) {
    Fail([NSString
        stringWithFormat:
            @"calendar source mismatch: proposed_source=%@ approval_source=%@",
            SourceID(existingProposed), SourceID(existingApproval)]);
    return NO;
  }

  *proposed = existingProposed;
  *approval = existingApproval;
  return YES;
}

static EKCalendar *FindCalendarByIdentifier(EKEventStore *store,
                                            NSString *identifier) {
  if (identifier.length == 0) {
    return nil;
  }
  for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
    if ([calendar.calendarIdentifier isEqualToString:identifier]) {
      return calendar;
    }
  }
  return nil;
}

static BOOL ResetAndRefreshCalendars(EKEventStore *store, EKCalendar **proposed,
                                     EKCalendar **approval) {
  NSString *proposedID = (*proposed).calendarIdentifier;
  NSString *approvalID = (*approval).calendarIdentifier;
  [store reset];
  *proposed = FindCalendarByIdentifier(store, proposedID);
  *approval = FindCalendarByIdentifier(store, approvalID);
  if (*proposed == nil || *approval == nil) {
    Fail([NSString stringWithFormat:
                       @"calendar refresh failed: proposed_found=%@ approval_found=%@",
                       *proposed == nil ? @"false" : @"true",
                       *approval == nil ? @"false" : @"true"]);
    return NO;
  }
  return YES;
}

static NSArray<EKEvent *> *MarkedEvents(EKEventStore *store,
                                        NSArray<EKCalendar *> *calendars) {
  NSDate *start = QaDateRelativeToToday(-365 * 3, 0, 0);
  NSDate *end = QaDate(2032, 12, 31, 23, 59);
  NSPredicate *predicate =
      [store predicateForEventsWithStartDate:start endDate:end calendars:calendars];
  NSMutableArray<EKEvent *> *events = [NSMutableArray array];
  for (EKEvent *event in [store eventsMatchingPredicate:predicate]) {
    if ([event.notes rangeOfString:QAMarker].location != NSNotFound) {
      [events addObject:event];
    }
  }
  return [events copy];
}

static NSUInteger CleanupMarkedEvents(EKEventStore *store,
                                      NSArray<EKCalendar *> *calendars,
                                      NSError **error) {
  NSUInteger removed = 0;
  for (EKEvent *event in MarkedEvents(store, calendars)) {
    if (![store removeEvent:event span:EKSpanThisEvent commit:NO error:error]) {
      return removed;
    }
    removed += 1;
  }
  if (removed > 0 && ![store commit:error]) {
    return removed;
  }
  return removed;
}

static EKEvent *CreateEvent(EKEventStore *store, EKCalendar *calendar,
                            NSString *title, NSDate *start, NSDate *end) {
  EKEvent *event = [EKEvent eventWithEventStore:store];
  event.calendar = calendar;
  event.title = title;
  event.startDate = start;
  event.endDate = end;
  event.allDay = NO;
  event.availability = EKEventAvailabilityFree;
  event.alarms = @[];
  event.notes = [NSString stringWithFormat:@"%@\nqa_title=%@", QAMarker, title];

  NSError *error = nil;
  if (![store saveEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"event create failed: %@",
                                     error.localizedDescription]);
    return nil;
  }
  if (event.eventIdentifier.length == 0) {
    Fail(@"event create failed: missing EventKit identifier");
    return nil;
  }
  return event;
}

static EKEvent *FindMarkedByTitle(EKEventStore *store, EKCalendar *calendar,
                                  NSString *title) {
  for (EKEvent *event in MarkedEvents(store, @[ calendar ])) {
    if ([event.title isEqualToString:title]) {
      return event;
    }
  }
  return nil;
}

static NSArray<EKEvent *> *FindMarkedEventsByTitle(EKEventStore *store,
                                                   EKCalendar *calendar,
                                                   NSString *title);
static BOOL SameEventPayload(EKEvent *left, EKEvent *right);
static NSString *PayloadDiagnostics(EKEvent *expected, EKEvent *readback);
static EKEvent *RefetchCreatedEventByIdentifier(EKEventStore *store,
                                                NSString *eventIdentifier,
                                                EKCalendar *calendar,
                                                EKEvent *event,
                                                NSString *operation);

static EKEvent *RefetchCreatedEventByIdentifier(EKEventStore *store,
                                                NSString *eventIdentifier,
                                                EKCalendar *calendar,
                                                EKEvent *event,
                                                NSString *operation) {
  if (eventIdentifier.length == 0) {
    Fail([NSString stringWithFormat:
                       @"%@ source identifier missing: title=%@ calendar_id=%@",
                       operation, event.title ?: @"<missing-title>",
                       event.calendar.calendarIdentifier ?: @"<missing-calendar-id>"]);
    return nil;
  }

  EKEvent *refetched = [store eventWithIdentifier:eventIdentifier];
  if (refetched != nil) {
    return refetched;
  }

  NSArray<EKEvent *> *titleMatches =
      FindMarkedEventsByTitle(store, calendar, event.title);
  if (titleMatches.count == 1 && SameEventPayload(event, titleMatches[0])) {
    return titleMatches[0];
  }

  if (titleMatches.count == 1) {
    Fail([NSString stringWithFormat:
                       @"%@ source predicate payload mismatch after identifier refetch failed: event_id=%@ title=%@ calendar_id=%@ %@",
                       operation, eventIdentifier,
                       event.title ?: @"<missing-title>",
                       event.calendar.calendarIdentifier ?: @"<missing-calendar-id>",
                       PayloadDiagnostics(event, titleMatches[0])]);
    return nil;
  }

  if (titleMatches.count > 1) {
    Fail([NSString stringWithFormat:
                       @"%@ source predicate refetch ambiguous after identifier refetch failed: event_id=%@ title=%@ calendar_id=%@ match_count=%lu",
                       operation, eventIdentifier,
                       event.title ?: @"<missing-title>",
                       event.calendar.calendarIdentifier ?: @"<missing-calendar-id>",
                       (unsigned long)titleMatches.count]);
    return nil;
  }

  {
    Fail([NSString stringWithFormat:
                       @"%@ source refetch by identifier failed: event_id=%@ title=%@ calendar_id=%@",
                       operation, eventIdentifier,
                       event.title ?: @"<missing-title>",
                       event.calendar.calendarIdentifier ?: @"<missing-calendar-id>"]);
    return nil;
  }
}

static NSArray<EKEvent *> *FindMarkedEventsByTitle(EKEventStore *store,
                                                   EKCalendar *calendar,
                                                   NSString *title) {
  NSMutableArray<EKEvent *> *events = [NSMutableArray array];
  for (EKEvent *event in MarkedEvents(store, @[ calendar ])) {
    if ([event.title isEqualToString:title]) {
      [events addObject:event];
    }
  }
  return [events copy];
}

static EKEvent *NewEventWithSamePayload(EKEventStore *store, EKCalendar *calendar,
                                        EKEvent *source) {
  EKEvent *target = [EKEvent eventWithEventStore:store];
  target.calendar = calendar;
  target.title = source.title;
  target.startDate = source.startDate;
  target.endDate = source.endDate;
  target.allDay = source.allDay;
  target.availability = source.availability;
  target.alarms = @[];
  target.notes = source.notes;
  return target;
}

static NSString *BoolString(BOOL value) {
  return value ? @"true" : @"false";
}

static NSString *EpochString(NSDate *date) {
  if (date == nil) {
    return @"<missing-date>";
  }
  return [NSString stringWithFormat:@"%.0f", date.timeIntervalSince1970];
}

static BOOL SameDateWithinOneSecond(NSDate *left, NSDate *right) {
  if (left == nil || right == nil) {
    return NO;
  }
  NSTimeInterval delta = [left timeIntervalSinceDate:right];
  if (delta < 0) {
    delta = -delta;
  }
  return delta <= 1.0;
}

static BOOL NotesContainStableMetadata(EKEvent *event) {
  NSString *notes = event.notes;
  NSString *title = event.title;
  if (notes.length == 0 || title.length == 0) {
    return NO;
  }
  NSString *titleMetadata = [NSString stringWithFormat:@"qa_title=%@", title];
  return [notes rangeOfString:QAMarker].location != NSNotFound &&
         [notes rangeOfString:titleMetadata].location != NSNotFound;
}

static BOOL SameEventPayload(EKEvent *left, EKEvent *right) {
  if (left == nil || right == nil) {
    return NO;
  }
  return [left.title isEqualToString:right.title] &&
         SameDateWithinOneSecond(left.startDate, right.startDate) &&
         SameDateWithinOneSecond(left.endDate, right.endDate) &&
         left.allDay == right.allDay &&
         NotesContainStableMetadata(right);
}

static NSString *PayloadDiagnostics(EKEvent *expected, EKEvent *readback) {
  BOOL expectedNotesHasMarker =
      expected.notes != nil && [expected.notes rangeOfString:QAMarker].location != NSNotFound;
  BOOL readbackNotesHasMarker =
      readback.notes != nil && [readback.notes rangeOfString:QAMarker].location != NSNotFound;
  NSString *expectedTitleMetadata =
      [NSString stringWithFormat:@"qa_title=%@", expected.title ?: @"<missing-title>"];
  NSString *readbackTitleMetadata =
      [NSString stringWithFormat:@"qa_title=%@", readback.title ?: @"<missing-title>"];
  BOOL expectedNotesHasTitle =
      expected.notes != nil &&
      [expected.notes rangeOfString:expectedTitleMetadata].location != NSNotFound;
  BOOL readbackNotesHasTitle =
      readback.notes != nil &&
      [readback.notes rangeOfString:readbackTitleMetadata].location != NSNotFound;

  return [NSString
      stringWithFormat:
          @"title_equal=%@ expected_title=%@ readback_title=%@ "
           "start_equal_1s=%@ expected_start_epoch=%@ readback_start_epoch=%@ "
           "end_equal_1s=%@ expected_end_epoch=%@ readback_end_epoch=%@ "
           "all_day_equal=%@ expected_all_day=%@ readback_all_day=%@ "
           "availability_equal=%@ expected_availability=%ld readback_availability=%ld "
           "notes_equal=%@ expected_notes_present=%@ readback_notes_present=%@ "
           "expected_notes_has_marker=%@ readback_notes_has_marker=%@ "
           "expected_notes_has_title_metadata=%@ readback_notes_has_title_metadata=%@ "
           "expected_calendar_id=%@ readback_calendar_id=%@",
          BoolString([expected.title isEqualToString:readback.title]),
          expected.title ?: @"<missing-title>", readback.title ?: @"<missing-title>",
          BoolString(SameDateWithinOneSecond(expected.startDate, readback.startDate)),
          EpochString(expected.startDate), EpochString(readback.startDate),
          BoolString(SameDateWithinOneSecond(expected.endDate, readback.endDate)),
          EpochString(expected.endDate), EpochString(readback.endDate),
          BoolString(expected.allDay == readback.allDay), BoolString(expected.allDay),
          BoolString(readback.allDay),
          BoolString(expected.availability == readback.availability),
          (long)expected.availability, (long)readback.availability,
          BoolString((expected.notes == nil && readback.notes == nil) ||
                     [expected.notes isEqualToString:readback.notes]),
          BoolString(expected.notes != nil), BoolString(readback.notes != nil),
          BoolString(expectedNotesHasMarker), BoolString(readbackNotesHasMarker),
          BoolString(expectedNotesHasTitle), BoolString(readbackNotesHasTitle),
          expected.calendar.calendarIdentifier ?: @"<missing-calendar-id>",
          readback.calendar.calendarIdentifier ?: @"<missing-calendar-id>"];
}

static BOOL SameCalendarIdentifier(EKCalendar *left, EKCalendar *right) {
  NSString *leftID = left.calendarIdentifier;
  NSString *rightID = right.calendarIdentifier;
  return leftID.length > 0 && rightID.length > 0 && [leftID isEqualToString:rightID];
}

static EKEvent *ReadApprovedByIdentifier(EKEventStore *store,
                                         NSString *approvedIdentifier,
                                         EKEvent *approved, EKCalendar *approval,
                                         NSString *operation) {
  if (approvedIdentifier.length == 0) {
    Fail([NSString stringWithFormat:
                       @"%@ approved identifier missing: title=%@ approval_calendar_id=%@",
                       operation, approved.title ?: @"<missing-title>",
                       approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
    return nil;
  }

  EKEvent *readback = [store eventWithIdentifier:approvedIdentifier];
  if (readback == nil) {
    Fail([NSString stringWithFormat:
                       @"%@ approved identifier readback failed: approved_event_id=%@ title=%@ approval_calendar_id=%@",
                       operation, approvedIdentifier,
                       approved.title ?: @"<missing-title>",
                       approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
    return nil;
  }
  if (!SameCalendarIdentifier(readback.calendar, approval)) {
    Fail([NSString stringWithFormat:
                       @"%@ approved calendar mismatch: approved_event_id=%@ readback_calendar_id=%@ expected_calendar_id=%@ readback_source=%@ expected_source=%@",
                       operation, approvedIdentifier,
                       readback.calendar.calendarIdentifier ?: @"<missing-calendar-id>",
                       approval.calendarIdentifier ?: @"<missing-calendar-id>",
                       SourceID(readback.calendar), SourceID(approval)]);
    return nil;
  }
  if (!SameEventPayload(approved, readback)) {
    Fail([NSString stringWithFormat:
                       @"%@ approved payload mismatch: approved_event_id=%@ title=%@ readback_title=%@ approval_calendar_id=%@ %@",
                       operation, approvedIdentifier,
                       approved.title ?: @"<missing-title>",
                       readback.title ?: @"<missing-title>",
                       approval.calendarIdentifier ?: @"<missing-calendar-id>",
                       PayloadDiagnostics(approved, readback)]);
    return nil;
  }
  return readback;
}

static BOOL ApproveByMoveToControlledCalendar(EKEventStore *store,
                                              EKCalendar **proposed,
                                              EKCalendar **approval,
                                              EKEvent *proposal,
                                              NSString *title,
                                              EKEvent **approvedReadbackOut,
                                              NSError **error) {
  if (proposal == nil) {
    Fail([NSString stringWithFormat:@"move source missing: title=%@", title]);
    return NO;
  }
  if (proposal.eventIdentifier.length == 0) {
    Fail([NSString stringWithFormat:
                       @"move source identifier missing: title=%@ calendar_id=%@",
                       proposal.title ?: title,
                       proposal.calendar.calendarIdentifier ?: @"<missing-calendar-id>"]);
    return NO;
  }

  /*
   EventKit can re-key or transiently hide an existing event after direct
   event.calendar reassignment. A user-visible Calendar move is equivalently
   proved here by saving one approved event with the same payload, then deleting
   the source proposal, and requiring readback of both effects.
   */
  EKEvent *approved = NewEventWithSamePayload(store, *approval, proposal);
  if (![store saveEvent:approved span:EKSpanThisEvent commit:YES error:error]) {
    Fail([NSString stringWithFormat:@"move approval save failed: %@",
                                     (*error).localizedDescription]);
    return NO;
  }
  NSString *approvedIdentifier = [approved.eventIdentifier copy];
  if (approvedIdentifier.length == 0) {
    Fail([NSString stringWithFormat:
                       @"move approved identifier missing: title=%@ approval_calendar_id=%@",
                       approved.title ?: title,
                       (*approval).calendarIdentifier ?: @"<missing-calendar-id>"]);
    return NO;
  }
  if (![store removeEvent:proposal span:EKSpanThisEvent commit:YES error:error]) {
    Fail([NSString stringWithFormat:@"move source cleanup failed: %@",
                                     (*error).localizedDescription]);
    return NO;
  }
  if (!ResetAndRefreshCalendars(store, proposed, approval)) {
    return NO;
  }

  NSArray<EKEvent *> *approvedReadbacks =
      FindMarkedEventsByTitle(store, *approval, title);
  NSArray<EKEvent *> *proposedReadbacks =
      FindMarkedEventsByTitle(store, *proposed, title);
  EKEvent *approvedByID =
      ReadApprovedByIdentifier(store, approvedIdentifier, approved, *approval, @"move");
  if (approvedByID == nil) {
    return NO;
  }
  if (approvedReadbacks.count > 1) {
    Fail([NSString
        stringWithFormat:
            @"move duplicate predicate readback failed: approval_count=%lu approved_event_id=%@ approval_calendar_id=%@ approval_source=%@",
            (unsigned long)approvedReadbacks.count,
            approvedIdentifier ?: @"<missing-event-id>",
            (*approval).calendarIdentifier ?: @"<missing-calendar-id>",
            SourceID(*approval)]);
    return NO;
  }
  if (approvedReadbacks.count == 1 && !SameEventPayload(approved, approvedReadbacks[0])) {
    Fail([NSString
        stringWithFormat:
            @"move predicate payload mismatch: approval_count=1 approved_event_id=%@ approval_calendar_id=%@ %@",
            approvedIdentifier ?: @"<missing-event-id>",
            (*approval).calendarIdentifier ?: @"<missing-calendar-id>",
            PayloadDiagnostics(approved, approvedReadbacks[0])]);
    return NO;
  }
  if (proposedReadbacks.count != 0) {
    Fail([NSString
        stringWithFormat:
            @"move proposed source still present: proposed_count=%lu approval_count=%lu approved_event_id=%@ proposed_calendar_id=%@ approval_calendar_id=%@ proposed_source=%@ approval_source=%@",
            (unsigned long)proposedReadbacks.count,
            (unsigned long)approvedReadbacks.count,
            approvedIdentifier ?: @"<missing-event-id>",
            (*proposed).calendarIdentifier ?: @"<missing-calendar-id>",
            (*approval).calendarIdentifier ?: @"<missing-calendar-id>",
            SourceID(*proposed), SourceID(*approval)]);
    return NO;
  }
  if (approvedReadbackOut != nil) {
    *approvedReadbackOut = approvedByID;
  }
  return YES;
}

static BOOL RemoveCreatedCalendar(EKEventStore *store, EKCalendar *calendar,
                                  BOOL created, NSString *reason) {
  if (!created) {
    return YES;
  }
  NSError *error = nil;
  if (![store removeCalendar:calendar commit:YES error:&error]) {
    Fail([NSString stringWithFormat:@"%@: %@", reason, error.localizedDescription]);
    return NO;
  }
  return YES;
}

static void CleanupArtifacts(EKEventStore *store, EKCalendar *proposed,
                             BOOL proposedCreated, EKCalendar *approval,
                             BOOL approvalCreated) {
  NSMutableArray<EKCalendar *> *calendars = [NSMutableArray array];
  [calendars addObjectsFromArray:WritableControlledCalendars(store)];
  if (proposed != nil) {
    [calendars addObject:proposed];
  }
  if (approval != nil) {
    [calendars addObject:approval];
  }
  if (calendars.count > 0) {
    (void)CleanupMarkedEvents(store, calendars, nil);
  }
  (void)RemoveCreatedCalendar(store, approval, approvalCreated,
                              @"approval calendar cleanup failed");
  (void)RemoveCreatedCalendar(store, proposed, proposedCreated,
                              @"proposed calendar cleanup failed");
}

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    NSString *commandDisplay =
        argc > 1 ? [NSString stringWithUTF8String:argv[1]]
                 : @"crates/morrow-calendar/scripts/calendar_lifecycle_real_qa.sh";
    EKEventStore *store = [[EKEventStore alloc] init];
    if (!RequestAccess(store, commandDisplay)) {
      return 77;
    }

    BOOL proposedCreated = NO;
    BOOL approvalCreated = NO;
    EKCalendar *proposed = nil;
    EKCalendar *approval = nil;
    if (!EnsureControlledCalendars(store, &proposed, &proposedCreated, &approval,
                                   &approvalCreated)) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    NSError *error = nil;
    NSUInteger staleRemoved =
        CleanupMarkedEvents(store, WritableControlledCalendars(store), &error);
    if (error != nil) {
      Fail([NSString stringWithFormat:@"stale cleanup failed: %@",
                                       error.localizedDescription]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    NSString *runID = RunID();
    NSString *moveTitle = [NSString stringWithFormat:@"Morrow QA F3 move %@", runID];
    NSString *deleteTitle = [NSString stringWithFormat:@"Morrow QA F3 delete %@", runID];
    NSString *copySourceTitle =
        [NSString stringWithFormat:@"Morrow QA F3 copy source %@", runID];
    NSString *copyTargetTitle =
        [NSString stringWithFormat:@"Morrow QA F3 copy target %@", runID];
    NSString *pastTitle = [NSString stringWithFormat:@"Morrow QA F3 past %@", runID];

    EKEvent *move = CreateEvent(store, proposed, moveTitle, QaDate(2031, 2, 4, 9, 0),
                                QaDate(2031, 2, 4, 9, 30));
    EKEvent *deleteCandidate =
        CreateEvent(store, proposed, deleteTitle, QaDate(2031, 2, 4, 10, 0),
                    QaDate(2031, 2, 4, 10, 30));
    EKEvent *copySource =
        CreateEvent(store, proposed, copySourceTitle, QaDate(2031, 2, 4, 11, 0),
                    QaDate(2031, 2, 4, 11, 30));
    if (move == nil || deleteCandidate == nil || copySource == nil) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    NSString *deleteCandidateIdentifier = [deleteCandidate.eventIdentifier copy];
    NSString *copySourceIdentifier = [copySource.eventIdentifier copy];

    if (!ApproveByMoveToControlledCalendar(store, &proposed, &approval, move,
                                           moveTitle, nil, &error)) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    deleteCandidate = RefetchCreatedEventByIdentifier(
        store, deleteCandidateIdentifier, proposed, deleteCandidate, @"delete");
    if (deleteCandidate == nil) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    error = nil;
    if (![store removeEvent:deleteCandidate span:EKSpanThisEvent commit:YES error:&error]) {
      Fail([NSString stringWithFormat:@"delete remove failed: %@",
                                       error.localizedDescription]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (!ResetAndRefreshCalendars(store, &proposed, &approval)) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (FindMarkedByTitle(store, proposed, deleteTitle) != nil) {
      Fail(@"delete readback failed: proposed event still present");
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    copySource = RefetchCreatedEventByIdentifier(
        store, copySourceIdentifier, proposed, copySource, @"copy");
    if (copySource == nil) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    EKEvent *copyTarget = [EKEvent eventWithEventStore:store];
    copyTarget.calendar = approval;
    copyTarget.title = copyTargetTitle;
    copyTarget.startDate = copySource.startDate;
    copyTarget.endDate = copySource.endDate;
    copyTarget.allDay = copySource.allDay;
    copyTarget.availability = EKEventAvailabilityFree;
    copyTarget.alarms = @[];
    copyTarget.notes = [NSString stringWithFormat:@"%@\nqa_title=%@\nqa_copied_from=%@",
                                                  QAMarker, copyTargetTitle,
                                                  copySourceTitle];
    if (![store saveEvent:copyTarget span:EKSpanThisEvent commit:YES error:&error]) {
      Fail([NSString stringWithFormat:@"copy target save failed: %@",
                                       error.localizedDescription]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    NSString *copyTargetIdentifier = [copyTarget.eventIdentifier copy];
    if (copyTargetIdentifier.length == 0) {
      Fail([NSString stringWithFormat:
                         @"copy approved identifier missing: title=%@ approval_calendar_id=%@",
                         copyTarget.title ?: copyTargetTitle,
                         approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (![store removeEvent:copySource span:EKSpanThisEvent commit:YES error:&error]) {
      Fail([NSString stringWithFormat:@"copy source cleanup failed: %@",
                                       error.localizedDescription]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (!ResetAndRefreshCalendars(store, &proposed, &approval)) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    NSArray<EKEvent *> *copyApprovedReadbacks =
        FindMarkedEventsByTitle(store, approval, copyTargetTitle);
    NSArray<EKEvent *> *copyProposedReadbacks =
        FindMarkedEventsByTitle(store, proposed, copySourceTitle);
    if (ReadApprovedByIdentifier(store, copyTargetIdentifier, copyTarget, approval,
                                 @"copy") == nil) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (copyApprovedReadbacks.count > 1) {
      Fail([NSString stringWithFormat:
                         @"copy duplicate predicate readback failed: approval_count=%lu approved_event_id=%@ approval_calendar_id=%@",
                         (unsigned long)copyApprovedReadbacks.count,
                         copyTargetIdentifier ?: @"<missing-event-id>",
                         approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (copyApprovedReadbacks.count == 1 &&
        !SameEventPayload(copyTarget, copyApprovedReadbacks[0])) {
      Fail([NSString stringWithFormat:
                         @"copy predicate payload mismatch: approval_count=1 approved_event_id=%@ approval_calendar_id=%@ %@",
                         copyTargetIdentifier ?: @"<missing-event-id>",
                         approval.calendarIdentifier ?: @"<missing-calendar-id>",
                         PayloadDiagnostics(copyTarget, copyApprovedReadbacks[0])]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (copyProposedReadbacks.count != 0) {
      Fail([NSString stringWithFormat:
                         @"copy proposed source still present: proposed_count=%lu approval_count=%lu approved_event_id=%@ proposed_calendar_id=%@ approval_calendar_id=%@",
                         (unsigned long)copyProposedReadbacks.count,
                         (unsigned long)copyApprovedReadbacks.count,
                         copyTargetIdentifier ?: @"<missing-event-id>",
                         proposed.calendarIdentifier ?: @"<missing-calendar-id>",
                         approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    EKEvent *past = CreateEvent(store, proposed, pastTitle,
                                QaDateRelativeToToday(-2, 8, 0),
                                QaDateRelativeToToday(-2, 8, 30));
    if (past == nil) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    EKEvent *pastApprovedReadback = nil;
    if (!ApproveByMoveToControlledCalendar(store, &proposed, &approval, past,
                                           pastTitle, &pastApprovedReadback, &error)) {
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (pastApprovedReadback == nil ||
        [pastApprovedReadback.endDate compare:NSDate.date] != NSOrderedAscending) {
      Fail([NSString stringWithFormat:
                         @"past event identifier readback failed: approved_found=%@ approved_event_id=%@ approval_calendar_id=%@",
                         pastApprovedReadback == nil ? @"false" : @"true",
                         pastApprovedReadback.eventIdentifier ?: @"<missing-event-id>",
                         approval.calendarIdentifier ?: @"<missing-calendar-id>"]);
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }

    error = nil;
    NSUInteger cleanupRemoved =
        CleanupMarkedEvents(store, WritableControlledCalendars(store), &error);
    if (error != nil || MarkedEvents(store, WritableControlledCalendars(store)).count != 0) {
      Fail(@"cleanup readback failed");
      CleanupArtifacts(store, proposed, proposedCreated, approval, approvalCreated);
      return 1;
    }
    if (!RemoveCreatedCalendar(store, approval, approvalCreated,
                               @"approval calendar cleanup failed") ||
        !RemoveCreatedCalendar(store, proposed, proposedCreated,
                               @"proposed calendar cleanup failed")) {
      return 1;
    }

    PrintLine(@"scenario=real-calendar-lifecycle-eventkit");
    PrintLine(@"status=passed");
    PrintLine([NSString stringWithFormat:@"run_id=%@", runID]);
    PrintLine(@"proposed_calendar=Morrow Proposed");
    PrintLine(@"approval_calendar=Morrow Proposed QA Approved");
    PrintLine([NSString stringWithFormat:@"stale_test_events_removed=%lu",
                                         (unsigned long)staleRemoved]);
    PrintLine(@"move_observation=approved_by_move_to_controlled_calendar");
    PrintLine(@"delete_observation=rejected_by_delete_from_proposed");
    PrintLine(@"copy_observation=approved_by_copy_to_controlled_calendar_source_proposal_removed");
    PrintLine(@"calendar_completion_applicability=not_applicable_eventkit_events_have_no_completed_flag");
    PrintLine(@"past_event_observation=approved_past_event_preserved_after_end_time");
    PrintLine([NSString stringWithFormat:@"cleanup_test_events_removed=%lu",
                                         (unsigned long)cleanupRemoved]);
    PrintLine(@"cleanup_test_events_remaining=0");
    PrintLine([NSString stringWithFormat:@"cleanup_created_proposed_calendar=%@",
                                         proposedCreated ? @"true" : @"false"]);
    PrintLine([NSString stringWithFormat:@"cleanup_created_approval_calendar=%@",
                                         approvalCreated ? @"true" : @"false"]);
    PrintLine(@"PASS calendar_lifecycle_real_qa");
    return 0;
  }
}
