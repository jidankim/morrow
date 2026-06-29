write_eventkit_helper() {
  local objc_source="$work_dir/morrow-real-qa-eventkit.m"
  cat > "$objc_source" <<'OBJC'
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

static BOOL authorize(EKEventStore *store) {
  EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
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
#endif
  __block BOOL granted = NO;
  dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
  if ([store respondsToSelector:@selector(requestFullAccessToEventsWithCompletion:)]) {
    [store requestFullAccessToEventsWithCompletion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  } else {
    [store requestAccessToEntityType:EKEntityTypeEvent completion:^(BOOL accessGranted, NSError *error) {
      (void)error;
      granted = accessGranted;
      dispatch_semaphore_signal(semaphore);
    }];
  }
  dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC));
  return granted;
}

static NSString *local_iso(NSDate *date) {
  NSDateFormatter *formatter = [[NSDateFormatter alloc] init];
  formatter.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
  formatter.dateFormat = @"yyyy-MM-dd'T'HH:mm:ss";
  return [formatter stringFromDate:date];
}

static int readback(NSString *eventId, NSString *expectedTitle, NSString *expectedStart) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_CALENDAR_ACCESS\n");
    return 20;
  }
  EKEvent *event = [store eventWithIdentifier:eventId];
  if (event == nil) {
    printf("{\"event_found\":false}\n");
    return 31;
  }
  BOOL titleOk = [event.title rangeOfString:expectedTitle options:NSCaseInsensitiveSearch].location != NSNotFound;
  BOOL startOk = [local_iso(event.startDate) isEqualToString:expectedStart];
  BOOL calendarOk = [event.calendar.title isEqualToString:@"Morrow Proposed"];
  BOOL availabilityOk = event.availability == EKEventAvailabilityFree;
  NSUInteger alarms = event.alarms == nil ? 0 : event.alarms.count;
  NSUInteger attendees = event.attendees == nil ? 0 : event.attendees.count;
  NSUInteger recurrence = event.recurrenceRules == nil ? 0 : event.recurrenceRules.count;
  printf("{\"event_found\":true,\"title_contains_expected\":%s,\"start_matches_expected\":%s,\"calendar_is_morrow_proposed\":%s,\"availability_free\":%s,\"alarms_count\":%lu,\"attendees_count\":%lu,\"recurrence_rule_count\":%lu}\n",
         titleOk ? "true" : "false",
         startOk ? "true" : "false",
         calendarOk ? "true" : "false",
         availabilityOk ? "true" : "false",
         (unsigned long)alarms,
         (unsigned long)attendees,
         (unsigned long)recurrence);
  return titleOk && startOk && calendarOk && availabilityOk && alarms == 0 && attendees == 0 && recurrence == 0 ? 0 : 32;
}

static int cleanup(NSString *eventId) {
  EKEventStore *store = [[EKEventStore alloc] init];
  if (!authorize(store)) {
    printf("BLOCKED_CALENDAR_ACCESS\n");
    return 20;
  }
  EKEvent *event = [store eventWithIdentifier:eventId];
  if (event == nil) {
    printf("CLEANUP_DELETED=false\n");
    return 0;
  }
  NSError *error = nil;
  BOOL removed = [store removeEvent:event span:EKSpanThisEvent commit:YES error:&error];
  (void)error;
  printf("CLEANUP_DELETED=%s\n", removed ? "true" : "false");
  return removed ? 0 : 40;
}

int main(int argc, const char * argv[]) {
  @autoreleasepool {
    if (argc < 3) {
      fprintf(stderr, "usage: eventkit-helper <readback|cleanup> <event-id> [expected-title] [expected-start]\n");
      return 64;
    }
    NSString *mode = [NSString stringWithUTF8String:argv[1]];
    NSString *eventId = [NSString stringWithUTF8String:argv[2]];
    if ([mode isEqualToString:@"readback"]) {
      if (argc != 5) {
        return 64;
      }
      return readback(eventId, [NSString stringWithUTF8String:argv[3]], [NSString stringWithUTF8String:argv[4]]);
    }
    if ([mode isEqualToString:@"cleanup"]) {
      return cleanup(eventId);
    }
    return 64;
  }
}
OBJC
  /usr/bin/clang -fobjc-arc -Wall -Wextra -framework Foundation -framework EventKit "$objc_source" -o "$eventkit_binary" > "$work_dir/eventkit-build.log" 2>&1
}
