write_eventkit_helper() {
  local objc_source="$work_dir/morrow-real-reminders-qa-eventkit.m"
  cat > "$objc_source" <<'OBJC'
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

static NSString *const ProposedName = @"Morrow Proposed";
static NSString *const MetadataBegin = @"[MORROW_METADATA_V1]";
static NSString *const MetadataEnd = @"[/MORROW_METADATA_V1]";

static BOOL authorize(EKEventStore *store) {
  EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
  if (status == EKAuthorizationStatusDenied || status == EKAuthorizationStatusRestricted) {
    return NO;
  }
  if (status == EKAuthorizationStatusAuthorized) {
    return YES;
  }
#ifdef EKAuthorizationStatusFullAccess
  if (status == EKAuthorizationStatusFullAccess) {
    return YES;
  }
  if (status == EKAuthorizationStatusWriteOnly) {
    return NO;
  }
#endif
  __block BOOL granted = NO;
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  if (@available(macOS 14.0, *)) {
    [store requestFullAccessToRemindersWithCompletion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  } else {
    [store requestAccessToEntityType:EKEntityTypeReminder completion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  }
  dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC));
  return granted;
}

static EKCalendar *proposed_list(EKEventStore *store) {
  for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeReminder]) {
    if ([calendar.title isEqualToString:ProposedName]) {
      return calendar;
    }
  }
  return nil;
}

static NSArray<EKReminder *> *fetch_proposed(EKEventStore *store, EKCalendar *calendar) {
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  NSPredicate *predicate = [store predicateForRemindersInCalendars:@[calendar]];
  __block NSArray<EKReminder *> *matched = @[];
  [store fetchRemindersMatchingPredicate:predicate completion:^(NSArray<EKReminder *> *reminders) {
    matched = [reminders copy] ?: @[];
    dispatch_semaphore_signal(semaphore);
  }];
  if (dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_SEC)) != 0) {
    return nil;
  }
  return matched;
}

static NSString *hex_encode(NSString *value) {
  NSData *data = [value dataUsingEncoding:NSUTF8StringEncoding];
  if (data == nil) {
    return @"";
  }
  const unsigned char *bytes = data.bytes;
  NSMutableString *encoded = [NSMutableString stringWithCapacity:data.length * 2];
  for (NSUInteger index = 0; index < data.length; index++) {
    [encoded appendFormat:@"%02x", bytes[index]];
  }
  return encoded;
}

static NSString *metadata_candidate_line(NSString *candidateId) {
  NSString *encoded = hex_encode(candidateId);
  if (encoded.length == 0) {
    return @"";
  }
  return [NSString stringWithFormat:@"candidate_id=%@", encoded];
}

static BOOL metadata_matches(EKReminder *reminder, NSString *candidateId) {
  NSString *candidateLine = metadata_candidate_line(candidateId);
  if (candidateLine.length == 0 || ![reminder.notes isKindOfClass:[NSString class]]) {
    return NO;
  }
  NSString *notes = reminder.notes;
  NSRange endRange = [notes rangeOfString:MetadataEnd options:NSBackwardsSearch];
  if (endRange.location == NSNotFound) {
    return NO;
  }
  NSRange searchRange = NSMakeRange(0, endRange.location);
  NSRange beginRange = [notes rangeOfString:MetadataBegin options:NSBackwardsSearch range:searchRange];
  if (beginRange.location == NSNotFound) {
    return NO;
  }
  NSUInteger bodyStart = beginRange.location + beginRange.length;
  NSString *body = [notes substringWithRange:NSMakeRange(bodyStart, endRange.location - bodyStart)];
  __block BOOL found = NO;
  [body enumerateLinesUsingBlock:^(NSString *line, BOOL *stop) {
    if ([line isEqualToString:candidateLine]) {
      found = YES;
      *stop = YES;
    }
  }];
  return found;
}

static EKReminder *find_reminder(EKEventStore *store, NSString *reminderId, NSString *candidateId, EKCalendar **calendarOut) {
  EKCalendar *calendar = proposed_list(store);
  if (calendarOut != NULL) {
    *calendarOut = calendar;
  }
  EKCalendarItem *item = [store calendarItemWithIdentifier:reminderId];
  if ([item isKindOfClass:[EKReminder class]]) {
    EKReminder *reminder = (EKReminder *)item;
    if ([reminder.calendar.title isEqualToString:ProposedName] || metadata_matches(reminder, candidateId)) {
      return reminder;
    }
  }
  if (calendar == nil) {
    return nil;
  }
  NSArray<EKReminder *> *reminders = fetch_proposed(store, calendar);
  for (EKReminder *reminder in reminders) {
    if ([reminder.calendarItemIdentifier isEqualToString:reminderId] || metadata_matches(reminder, candidateId)) {
      return reminder;
    }
  }
  return nil;
}

static NSArray<NSString *> *parts(NSString *value, NSString *separator) {
  return [value componentsSeparatedByString:separator];
}

static BOOL due_matches(NSDateComponents *due, NSString *expectedLocal) {
  NSArray<NSString *> *dateTime = parts(expectedLocal, @"T");
  if (dateTime.count != 2) {
    return NO;
  }
  NSArray<NSString *> *date = parts(dateTime[0], @"-");
  NSArray<NSString *> *time = parts(dateTime[1], @":");
  if (date.count != 3 || time.count != 3) {
    return NO;
  }
  return due.year == [date[0] integerValue]
      && due.month == [date[1] integerValue]
      && due.day == [date[2] integerValue]
      && due.hour == [time[0] integerValue]
      && due.minute == [time[1] integerValue]
      && due.second == [time[2] integerValue];
}

static int readback(NSString *reminderId, NSString *candidateId, NSString *expectedTitle, NSString *expectedLocal) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_REMINDERS_ACCESS\n");
    return 20;
  }
  EKCalendar *calendar = nil;
  EKReminder *reminder = find_reminder(store, reminderId, candidateId, &calendar);
  if (reminder == nil) {
    printf("{\"reminder_found\":false}\n");
    return 31;
  }
  BOOL titleOk = [reminder.title rangeOfString:expectedTitle options:NSCaseInsensitiveSearch].location != NSNotFound;
  BOOL listOk = [reminder.calendar.title isEqualToString:ProposedName];
  BOOL dueOk = reminder.dueDateComponents != nil && due_matches(reminder.dueDateComponents, expectedLocal);
  BOOL metadataOk = metadata_matches(reminder, candidateId);
  printf("{\"reminder_found\":true,\"title_contains_expected\":%s,\"list_is_morrow_proposed\":%s,\"due_matches_expected\":%s,\"metadata_markers_present\":%s}\n",
         titleOk ? "true" : "false",
         listOk ? "true" : "false",
         dueOk ? "true" : "false",
         metadataOk ? "true" : "false");
  return titleOk && listOk && dueOk && metadataOk && calendar != nil ? 0 : 32;
}

static int cleanup(NSString *reminderId, NSString *candidateId) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_REMINDERS_ACCESS\n");
    return 20;
  }
  EKCalendar *calendar = nil;
  EKReminder *reminder = find_reminder(store, reminderId, candidateId, &calendar);
  BOOL removed = NO;
  if (reminder != nil) {
    NSError *error = nil;
    removed = [store removeReminder:reminder commit:YES error:&error];
    (void)error;
    if (!removed) {
      printf("CLEANUP_DELETED=false\n");
      return 40;
    }
  }
  NSArray<EKReminder *> *remaining = calendar == nil ? @[] : fetch_proposed(store, calendar);
  for (EKReminder *candidate in remaining) {
    if ([candidate.calendarItemIdentifier isEqualToString:reminderId] || metadata_matches(candidate, candidateId)) {
      printf("CLEANUP_DELETED=false\n");
      return 41;
    }
  }
  printf("CLEANUP_DELETED=%s\n", removed ? "true" : "false");
  return 0;
}

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    if (argc < 4) {
      fprintf(stderr, "usage: eventkit-helper <readback|cleanup> <reminder-id> <candidate-id> [expected-title] [expected-local]\n");
      return 64;
    }
    NSString *mode = [NSString stringWithUTF8String:argv[1]];
    NSString *reminderId = [NSString stringWithUTF8String:argv[2]];
    NSString *candidateId = [NSString stringWithUTF8String:argv[3]];
    if ([mode isEqualToString:@"readback"]) {
      if (argc != 6) {
        return 64;
      }
      return readback(reminderId, candidateId, [NSString stringWithUTF8String:argv[4]], [NSString stringWithUTF8String:argv[5]]);
    }
    if ([mode isEqualToString:@"cleanup"]) {
      return cleanup(reminderId, candidateId);
    }
    return 64;
  }
}
OBJC
  /usr/bin/clang -fobjc-arc -Wall -Wextra -framework Foundation -framework EventKit "$objc_source" -o "$eventkit_binary" > "$work_dir/eventkit-build.log" 2>&1
}
