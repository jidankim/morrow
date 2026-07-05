#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <stdint.h>
#include <string.h>

// allow: SIZE_OK - This pre-existing Objective-C EventKit bridge keeps the C
// ABI structs, Reminders authorization flow, proposed-list lookup, metadata
// dedupe, and save/result mapping in one ARC translation unit. Splitting it in
// the T4 repair would risk native symbol linkage, block lifetime, and
// permission/list-store behavior before production replay QA covers the path.
// Follow-up risk: split authorization/list resolution and dedupe helpers after
// T5-T7 prove the Reminders replay ABI and cleanup behavior end to end.

typedef struct {
    const char *title;
    const char *notes;
    int64_t start_unix;
    int64_t end_unix;
} MorrowEventKitProposalRequest;

typedef struct {
    const char *title;
    const char *notes;
    const char *metadata_candidate_id;
    int due_year;
    int due_month;
    int due_day;
    int has_due_time;
    int due_hour;
    int due_minute;
    int due_second;
    const char *timezone_name;
} MorrowEventKitReminderRequest;

typedef struct {
    int ok;
    int error_code;
    int truncated_field;
    char event_id[256];
    char calendar_id[256];
    char source_id[256];
    char message[512];
} MorrowEventKitProposalResult;

typedef struct {
    int ok;
    int error_code;
    int truncated_field;
    char reminder_id[256];
    char list_id[256];
    char source_id[256];
    char message[512];
} MorrowEventKitReminderResult;

static const int ErrorPermissionDenied = 1;
static const int ErrorSourceUnavailable = 2;
static const int ErrorSaveFailed = 3;
static const int ErrorEmptyEventIdentifier = 4;
static const int ErrorUnavailable = 5;
static const int TruncatedEventId = 1;
static const int TruncatedCalendarId = 2;
static const int TruncatedSourceId = 3;
static NSString *const ProposedName = @"Morrow Proposed";
static NSString *const MetadataBegin = @"[MORROW_METADATA_V1]";
static NSString *const MetadataEnd = @"[/MORROW_METADATA_V1]";

static BOOL SetString(char *buffer, size_t capacity, NSString *value) {
    const char *text = [value ?: @"" UTF8String];
    return strlcpy(buffer, text, capacity) < capacity;
}

static void SetError(MorrowEventKitProposalResult *result, int code, NSString *message) {
    if (result == NULL) {
        return;
    }
    result->ok = 0;
    result->error_code = code;
    (void)SetString(result->message, sizeof(result->message), message);
}

static void SetReminderError(MorrowEventKitReminderResult *result, int code, NSString *message) {
    if (result == NULL) {
        return;
    }
    result->ok = 0;
    result->error_code = code;
    (void)SetString(result->message, sizeof(result->message), message);
}

static BOOL SetIdentifier(char *buffer, size_t capacity, NSString *value, int truncatedField, MorrowEventKitProposalResult *result) {
    if (SetString(buffer, capacity, value)) {
        return YES;
    }
    result->truncated_field = truncatedField;
    SetError(result, ErrorSaveFailed, @"EventKit returned an identifier that is too long");
    return NO;
}

static BOOL SetReminderIdentifier(char *buffer, size_t capacity, NSString *value, int truncatedField, MorrowEventKitReminderResult *result) {
    if (SetString(buffer, capacity, value)) {
        return YES;
    }
    result->truncated_field = truncatedField;
    SetReminderError(result, ErrorSaveFailed, @"EventKit returned an identifier that is too long");
    return NO;
}

static void RequestLegacyEventAccess(EKEventStore *store, void (^completion)(BOOL, NSError *)) {
    SEL selector = NSSelectorFromString(@"requestAccessToEntityType:completion:");
    NSMethodSignature *signature = [store methodSignatureForSelector:selector];
    if (signature == nil) {
        completion(NO, nil);
        return;
    }
    NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
    invocation.target = store;
    invocation.selector = selector;
    EKEntityType type = EKEntityTypeEvent;
    void (^completionCopy)(BOOL, NSError *) = [completion copy];
    [invocation setArgument:&type atIndex:2];
    [invocation setArgument:&completionCopy atIndex:3];
    [invocation invoke];
}

static BOOL RequestEventAccess(EKEventStore *store, MorrowEventKitProposalResult *result) {
    if (store == nil) {
        SetError(result, ErrorUnavailable, @"EventKit store is unavailable");
        return NO;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block BOOL granted = NO;
    __block NSError *requestError = nil;

    if (@available(macOS 14.0, *)) {
        [store requestFullAccessToEventsWithCompletion:^(BOOL ok, NSError *error) {
            granted = ok;
            requestError = error;
            dispatch_semaphore_signal(semaphore);
        }];
    } else {
        RequestLegacyEventAccess(store, ^(BOOL ok, NSError *error) {
            granted = ok;
            requestError = error;
            dispatch_semaphore_signal(semaphore);
        });
    }

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 120LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        SetError(result, ErrorPermissionDenied, @"Calendar permission prompt did not resolve");
        return NO;
    }
    if (requestError != nil) {
        SetError(result, ErrorPermissionDenied, [NSString stringWithFormat:@"EventKit access request failed: %@", requestError.localizedDescription]);
        return NO;
    }
    if (!granted) {
        SetError(result, ErrorPermissionDenied, @"Calendar access was denied");
        return NO;
    }
    return YES;
}

static void RequestLegacyReminderAccess(EKEventStore *store, void (^completion)(BOOL, NSError *)) {
    SEL selector = NSSelectorFromString(@"requestAccessToEntityType:completion:");
    NSMethodSignature *signature = [store methodSignatureForSelector:selector];
    if (signature == nil) {
        completion(NO, nil);
        return;
    }
    NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
    invocation.target = store;
    invocation.selector = selector;
    EKEntityType type = EKEntityTypeReminder;
    void (^completionCopy)(BOOL, NSError *) = [completion copy];
    [invocation setArgument:&type atIndex:2];
    [invocation setArgument:&completionCopy atIndex:3];
    [invocation invoke];
}

static BOOL RequestReminderAccess(EKEventStore *store, MorrowEventKitReminderResult *result) {
    if (store == nil) {
        SetReminderError(result, ErrorUnavailable, @"EventKit store is unavailable");
        return NO;
    }

    EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
    if (status == EKAuthorizationStatusDenied) {
        SetReminderError(result, ErrorPermissionDenied, @"Reminders access was denied");
        return NO;
    }
    if (status == EKAuthorizationStatusRestricted) {
        SetReminderError(result, ErrorPermissionDenied, @"Reminders access is restricted");
        return NO;
    }
    if (@available(macOS 14.0, *)) {
        if (status == EKAuthorizationStatusWriteOnly) {
            SetReminderError(result, ErrorPermissionDenied, @"Reminders write-only access cannot read Morrow metadata");
            return NO;
        }
    }
    if (status != EKAuthorizationStatusNotDetermined) {
        return YES;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block BOOL granted = NO;
    __block NSError *requestError = nil;

    if (@available(macOS 14.0, *)) {
        [store requestFullAccessToRemindersWithCompletion:^(BOOL ok, NSError *error) {
            granted = ok;
            requestError = error;
            dispatch_semaphore_signal(semaphore);
        }];
    } else {
        RequestLegacyReminderAccess(store, ^(BOOL ok, NSError *error) {
            granted = ok;
            requestError = error;
            dispatch_semaphore_signal(semaphore);
        });
    }

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 120LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        SetReminderError(result, ErrorPermissionDenied, @"Reminders permission prompt did not resolve");
        return NO;
    }
    if (requestError != nil) {
        SetReminderError(result, ErrorPermissionDenied, [NSString stringWithFormat:@"EventKit access request failed: %@", requestError.localizedDescription]);
        return NO;
    }
    if (!granted) {
        SetReminderError(result, ErrorPermissionDenied, @"Reminders access was denied");
        return NO;
    }
    EKAuthorizationStatus resolvedStatus = [EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
    if (@available(macOS 14.0, *)) {
        if (resolvedStatus == EKAuthorizationStatusWriteOnly) {
            SetReminderError(result, ErrorPermissionDenied, @"Reminders write-only access cannot read Morrow metadata");
            return NO;
        }
    }
    return YES;
}

static NSString *CalendarTitle(EKCalendar *calendar) {
    if (![calendar isKindOfClass:[EKCalendar class]]) {
        return @"";
    }
    NSString *title = calendar.title;
    return [title isKindOfClass:[NSString class]] ? title : @"";
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

static BOOL HasSourceIdentifier(EKCalendar *calendar) {
    NSString *sourceIdentifier = calendar.source.sourceIdentifier;
    return [sourceIdentifier isKindOfClass:[NSString class]] && sourceIdentifier.length > 0;
}

static BOOL HasCalendarIdentifier(EKCalendar *calendar) {
    NSString *calendarIdentifier = calendar.calendarIdentifier;
    return [calendarIdentifier isKindOfClass:[NSString class]] && calendarIdentifier.length > 0;
}

static EKCalendar *EnsureProposedCalendar(EKEventStore *store, MorrowEventKitProposalResult *result) {
    for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeEvent]) {
        if ([CalendarTitle(calendar) isEqualToString:ProposedName]) {
            if (!calendar.allowsContentModifications) {
                SetError(result, ErrorSourceUnavailable, @"Morrow Proposed exists but is read-only");
                return nil;
            }
            if (!HasSourceIdentifier(calendar) || !HasCalendarIdentifier(calendar)) {
                SetError(result, ErrorSourceUnavailable, @"Morrow Proposed is missing an EventKit source identifier");
                return nil;
            }
            return calendar;
        }
    }

    EKSource *source = WritableSource(store);
    if (source == nil || source.sourceIdentifier.length == 0) {
        SetError(result, ErrorSourceUnavailable, @"No writable EventKit source is available for Morrow Proposed");
        return nil;
    }

    EKCalendar *calendar = [EKCalendar calendarForEntityType:EKEntityTypeEvent eventStore:store];
    calendar.title = ProposedName;
    calendar.source = source;
    NSError *error = nil;
    if (![store saveCalendar:calendar commit:YES error:&error]) {
        SetError(result, ErrorSourceUnavailable, [NSString stringWithFormat:@"Morrow Proposed calendar creation failed: %@", error.localizedDescription ?: @"unknown EventKit error"]);
        return nil;
    }
    if (!HasCalendarIdentifier(calendar) || !HasSourceIdentifier(calendar)) {
        SetError(result, ErrorSourceUnavailable, @"Morrow Proposed did not receive EventKit calendar/source identifiers");
        return nil;
    }
    return calendar;
}

static EKSource *WritableReminderSource(EKEventStore *store) {
    if (store.defaultCalendarForNewReminders.source != nil) {
        return store.defaultCalendarForNewReminders.source;
    }
    for (EKSource *source in store.sources) {
        if (source.sourceType == EKSourceTypeLocal) {
            return source;
        }
    }
    return store.sources.firstObject;
}

static EKCalendar *EnsureProposedReminderList(EKEventStore *store, MorrowEventKitReminderResult *result) {
    for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeReminder]) {
        if ([CalendarTitle(calendar) isEqualToString:ProposedName]) {
            if (!calendar.allowsContentModifications) {
                SetReminderError(result, ErrorSourceUnavailable, @"Morrow Proposed exists but is read-only");
                return nil;
            }
            if (!HasSourceIdentifier(calendar) || !HasCalendarIdentifier(calendar)) {
                SetReminderError(result, ErrorSourceUnavailable, @"Morrow Proposed is missing an EventKit source identifier");
                return nil;
            }
            return calendar;
        }
    }

    EKSource *source = WritableReminderSource(store);
    if (source == nil || source.sourceIdentifier.length == 0) {
        SetReminderError(result, ErrorSourceUnavailable, @"No writable EventKit source is available for Morrow Proposed");
        return nil;
    }

    EKCalendar *calendar = [EKCalendar calendarForEntityType:EKEntityTypeReminder eventStore:store];
    calendar.title = ProposedName;
    calendar.source = source;
    NSError *error = nil;
    if (![store saveCalendar:calendar commit:YES error:&error]) {
        SetReminderError(result, ErrorSourceUnavailable, [NSString stringWithFormat:@"Morrow Proposed reminder list creation failed: %@", error.localizedDescription ?: @"unknown EventKit error"]);
        return nil;
    }
    if (!HasCalendarIdentifier(calendar) || !HasSourceIdentifier(calendar)) {
        SetReminderError(result, ErrorSourceUnavailable, @"Morrow Proposed did not receive EventKit reminder list/source identifiers");
        return nil;
    }
    return calendar;
}

static NSString *RequestString(const char *value) {
    if (value == NULL) {
        return nil;
    }
    return [NSString stringWithUTF8String:value];
}

static NSArray<EKReminder *> *FetchReminders(EKEventStore *store, EKCalendar *calendar, MorrowEventKitReminderResult *result) {
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    NSPredicate *predicate = [store predicateForRemindersInCalendars:@[calendar]];
    __block NSArray<EKReminder *> *matched = @[];

    [store fetchRemindersMatchingPredicate:predicate completion:^(NSArray<EKReminder *> *reminders) {
        matched = [reminders copy] ?: @[];
        dispatch_semaphore_signal(semaphore);
    }];

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        SetReminderError(result, ErrorSaveFailed, @"Reminders readback timed out");
        return nil;
    }
    return matched;
}

static NSDateComponents *ReminderDueComponents(const MorrowEventKitReminderRequest *request, MorrowEventKitReminderResult *result) {
    NSDateComponents *components = [[NSDateComponents alloc] init];
    components.calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
    components.year = request->due_year;
    components.month = request->due_month;
    components.day = request->due_day;
    if (request->has_due_time != 0) {
        components.hour = request->due_hour;
        components.minute = request->due_minute;
        components.second = request->due_second;
    }
    NSString *timezoneName = RequestString(request->timezone_name);
    if (timezoneName.length > 0) {
        NSTimeZone *timezone = [NSTimeZone timeZoneWithName:timezoneName];
        if (timezone == nil) {
            SetReminderError(result, ErrorSaveFailed, @"Reminder timezone is unsupported by EventKit");
            return nil;
        }
        components.timeZone = timezone;
    }
    return components;
}

static BOOL ReminderMetadataContainsCandidate(EKReminder *reminder, NSString *candidateLine) {
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

static NSString *SaveProposalReminder(EKEventStore *store, EKCalendar *calendar, const MorrowEventKitReminderRequest *request, MorrowEventKitReminderResult *result) {
    NSString *title = RequestString(request->title);
    NSString *notes = RequestString(request->notes);
    NSString *metadataCandidateId = RequestString(request->metadata_candidate_id);
    if (title == nil || notes == nil) {
        SetReminderError(result, ErrorSaveFailed, @"Reminder proposal title or notes were not valid UTF-8");
        return nil;
    }
    if (metadataCandidateId.length == 0) {
        SetReminderError(result, ErrorSaveFailed, @"Reminder proposal metadata candidate id is missing");
        return nil;
    }
    NSDateComponents *due = ReminderDueComponents(request, result);
    if (due == nil) {
        return nil;
    }

    NSArray<EKReminder *> *existingReminders = FetchReminders(store, calendar, result);
    if (existingReminders == nil) {
        return nil;
    }
    for (EKReminder *existing in existingReminders) {
        if (ReminderMetadataContainsCandidate(existing, metadataCandidateId) &&
            existing.calendarItemIdentifier.length > 0) {
            return existing.calendarItemIdentifier;
        }
    }

    EKReminder *reminder = [EKReminder reminderWithEventStore:store];
    reminder.calendar = calendar;
    reminder.title = title;
    reminder.notes = notes;
    reminder.dueDateComponents = due;

    NSError *error = nil;
    if (![store saveReminder:reminder commit:YES error:&error]) {
        SetReminderError(result, ErrorSaveFailed, [NSString stringWithFormat:@"EventKit saveReminder failed: %@", error.localizedDescription ?: @"unknown EventKit error"]);
        return nil;
    }
    if (reminder.calendarItemIdentifier.length == 0) {
        SetReminderError(result, ErrorEmptyEventIdentifier, @"EventKit did not return a reminder identifier");
        return nil;
    }
    return reminder.calendarItemIdentifier;
}

static NSString *SaveProposalEvent(EKEventStore *store, EKCalendar *calendar, const MorrowEventKitProposalRequest *request, MorrowEventKitProposalResult *result) {
    NSString *title = RequestString(request->title);
    NSString *notes = RequestString(request->notes);
    if (title == nil || notes == nil) {
        SetError(result, ErrorSaveFailed, @"Calendar proposal title or notes were not valid UTF-8");
        return nil;
    }

    NSDate *startDate = [NSDate dateWithTimeIntervalSince1970:(NSTimeInterval)request->start_unix];
    NSDate *endDate = [NSDate dateWithTimeIntervalSince1970:(NSTimeInterval)request->end_unix];
    NSPredicate *predicate = [store predicateForEventsWithStartDate:startDate endDate:endDate calendars:@[calendar]];
    for (EKEvent *existing in [store eventsMatchingPredicate:predicate]) {
        if ([existing.title isEqualToString:title] &&
            [existing.startDate isEqualToDate:startDate] &&
            [existing.endDate isEqualToDate:endDate] &&
            [existing.notes isEqualToString:notes] &&
            existing.eventIdentifier.length > 0) {
            return existing.eventIdentifier;
        }
    }

    EKEvent *event = [EKEvent eventWithEventStore:store];
    event.calendar = calendar;
    event.title = title;
    event.startDate = startDate;
    event.endDate = endDate;
    event.allDay = NO;
    event.availability = EKEventAvailabilityFree;
    event.alarms = @[];
    event.notes = notes;

    NSError *error = nil;
    if (![store saveEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
        SetError(result, ErrorSaveFailed, [NSString stringWithFormat:@"EventKit saveEvent failed: %@", error.localizedDescription ?: @"unknown EventKit error"]);
        return nil;
    }
    if (event.eventIdentifier.length == 0) {
        SetError(result, ErrorEmptyEventIdentifier, @"EventKit did not return an event identifier");
        return nil;
    }
    return event.eventIdentifier;
}

void morrow_eventkit_create_proposal_event(const MorrowEventKitProposalRequest *request, MorrowEventKitProposalResult *result) {
    @autoreleasepool {
        if (result == NULL) {
            return;
        }
        memset(result, 0, sizeof(*result));
        if (request == NULL) {
            SetError(result, ErrorSaveFailed, @"Calendar proposal request is missing");
            return;
        }

        EKEventStore *store = [[EKEventStore alloc] init];
        if (!RequestEventAccess(store, result)) {
            return;
        }
        EKCalendar *calendar = EnsureProposedCalendar(store, result);
        if (calendar == nil) {
            return;
        }
        NSString *eventIdentifier = SaveProposalEvent(store, calendar, request, result);
        if (eventIdentifier == nil) {
            return;
        }

        if (!SetIdentifier(result->event_id, sizeof(result->event_id), eventIdentifier, TruncatedEventId, result)) {
            return;
        }
        if (!SetIdentifier(result->calendar_id, sizeof(result->calendar_id), calendar.calendarIdentifier, TruncatedCalendarId, result)) {
            return;
        }
        if (!SetIdentifier(result->source_id, sizeof(result->source_id), calendar.source.sourceIdentifier, TruncatedSourceId, result)) {
            return;
        }
        result->ok = 1;
    }
}

void morrow_eventkit_create_proposal_reminder(const MorrowEventKitReminderRequest *request, MorrowEventKitReminderResult *result) {
    @autoreleasepool {
        if (result == NULL) {
            return;
        }
        memset(result, 0, sizeof(*result));
        if (request == NULL) {
            SetReminderError(result, ErrorSaveFailed, @"Reminder proposal request is missing");
            return;
        }

        EKEventStore *store = [[EKEventStore alloc] init];
        if (!RequestReminderAccess(store, result)) {
            return;
        }
        EKCalendar *calendar = EnsureProposedReminderList(store, result);
        if (calendar == nil) {
            return;
        }
        NSString *reminderIdentifier = SaveProposalReminder(store, calendar, request, result);
        if (reminderIdentifier == nil) {
            return;
        }

        if (!SetReminderIdentifier(result->reminder_id, sizeof(result->reminder_id), reminderIdentifier, TruncatedEventId, result)) {
            return;
        }
        if (!SetReminderIdentifier(result->list_id, sizeof(result->list_id), calendar.calendarIdentifier, TruncatedCalendarId, result)) {
            return;
        }
        if (!SetReminderIdentifier(result->source_id, sizeof(result->source_id), calendar.source.sourceIdentifier, TruncatedSourceId, result)) {
            return;
        }
        result->ok = 1;
    }
}
