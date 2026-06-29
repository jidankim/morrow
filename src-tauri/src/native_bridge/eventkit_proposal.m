#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    const char *title;
    const char *notes;
    int64_t start_unix;
    int64_t end_unix;
} MorrowEventKitProposalRequest;

typedef struct {
    int ok;
    int error_code;
    int truncated_field;
    char event_id[256];
    char calendar_id[256];
    char source_id[256];
    char message[512];
} MorrowEventKitProposalResult;

static const int ErrorPermissionDenied = 1;
static const int ErrorSourceUnavailable = 2;
static const int ErrorSaveFailed = 3;
static const int ErrorEmptyEventIdentifier = 4;
static const int ErrorUnavailable = 5;
static const int TruncatedEventId = 1;
static const int TruncatedCalendarId = 2;
static const int TruncatedSourceId = 3;
static NSString *const ProposedName = @"Morrow Proposed";

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

static BOOL SetIdentifier(char *buffer, size_t capacity, NSString *value, int truncatedField, MorrowEventKitProposalResult *result) {
    if (SetString(buffer, capacity, value)) {
        return YES;
    }
    result->truncated_field = truncatedField;
    SetError(result, ErrorSaveFailed, @"EventKit returned an identifier that is too long");
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

static NSString *RequestString(const char *value) {
    if (value == NULL) {
        return nil;
    }
    return [NSString stringWithUTF8String:value];
}

static NSString *SaveProposalEvent(EKEventStore *store, EKCalendar *calendar, const MorrowEventKitProposalRequest *request, MorrowEventKitProposalResult *result) {
    NSString *title = RequestString(request->title);
    NSString *notes = RequestString(request->notes);
    if (title == nil || notes == nil) {
        SetError(result, ErrorSaveFailed, @"Calendar proposal title or notes were not valid UTF-8");
        return nil;
    }

    EKEvent *event = [EKEvent eventWithEventStore:store];
    event.calendar = calendar;
    event.title = title;
    event.startDate = [NSDate dateWithTimeIntervalSince1970:(NSTimeInterval)request->start_unix];
    event.endDate = [NSDate dateWithTimeIntervalSince1970:(NSTimeInterval)request->end_unix];
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
