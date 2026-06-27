#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    int ok;
    uint64_t calendar_items_deleted;
    uint64_t reminder_items_deleted;
    char message[512];
} MorrowEventKitCleanupResult;

static NSString *const ProposedName = @"Morrow Proposed";

static void SetMessage(MorrowEventKitCleanupResult *result, NSString *message) {
    if (result == NULL) {
        return;
    }
    const char *text = [message ?: @"Morrow proposed item cleanup failed" UTF8String];
    strlcpy(result->message, text, sizeof(result->message));
}

static BOOL RequestAccess(EKEventStore *store, EKEntityType type, MorrowEventKitCleanupResult *result) {
    if (store == nil) {
        SetMessage(result, @"EventKit store is unavailable");
        return NO;
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block BOOL granted = NO;
    __block NSError *requestError = nil;

    if (@available(macOS 14.0, *)) {
        if (type == EKEntityTypeEvent) {
            [store requestFullAccessToEventsWithCompletion:^(BOOL ok, NSError *error) {
                granted = ok;
                requestError = error;
                dispatch_semaphore_signal(semaphore);
            }];
        } else {
            [store requestFullAccessToRemindersWithCompletion:^(BOOL ok, NSError *error) {
                granted = ok;
                requestError = error;
                dispatch_semaphore_signal(semaphore);
            }];
        }
    } else {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        [store requestAccessToEntityType:type completion:^(BOOL ok, NSError *error) {
            granted = ok;
            requestError = error;
            dispatch_semaphore_signal(semaphore);
        }];
#pragma clang diagnostic pop
    }

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 120LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        SetMessage(result, @"EventKit permission prompt did not resolve");
        return NO;
    }
    if (requestError != nil) {
        SetMessage(result, [NSString stringWithFormat:@"EventKit access request failed: %@", requestError.localizedDescription]);
        return NO;
    }
    if (!granted) {
        SetMessage(result, type == EKEntityTypeEvent ? @"Calendar access was denied" : @"Reminders access was denied");
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

static EKCalendar *ContainerNamed(EKEventStore *store, EKEntityType type, NSString *name) {
    for (EKCalendar *calendar in [store calendarsForEntityType:type]) {
        if ([CalendarTitle(calendar) isEqualToString:name]) {
            return calendar;
        }
    }
    return nil;
}

static uint64_t RemoveEventsInProposedCalendar(EKEventStore *store, MorrowEventKitCleanupResult *result) {
    EKCalendar *calendar = ContainerNamed(store, EKEntityTypeEvent, ProposedName);
    if (calendar == nil) {
        return 0;
    }

    NSDate *start = [NSDate dateWithTimeIntervalSinceNow:-366 * 24 * 60 * 60];
    NSDate *end = [NSDate dateWithTimeIntervalSinceNow:366 * 5 * 24 * 60 * 60];
    NSPredicate *predicate = [store predicateForEventsWithStartDate:start endDate:end calendars:@[calendar]];
    NSArray<EKEvent *> *events = [store eventsMatchingPredicate:predicate];
    uint64_t removed = 0;
    for (EKEvent *event in events) {
        NSError *error = nil;
        if (![store removeEvent:event span:EKSpanThisEvent commit:YES error:&error]) {
            SetMessage(result, [NSString stringWithFormat:@"Calendar proposed item cleanup failed: %@", error.localizedDescription]);
            return removed;
        }
        removed += 1;
    }
    return removed;
}

static NSArray<EKReminder *> *FetchReminders(EKEventStore *store, EKCalendar *list, MorrowEventKitCleanupResult *result) {
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    NSPredicate *predicate = [store predicateForRemindersInCalendars:@[list]];
    __block NSArray<EKReminder *> *matched = @[];

    [store fetchRemindersMatchingPredicate:predicate completion:^(NSArray<EKReminder *> *reminders) {
        matched = [reminders copy] ?: @[];
        dispatch_semaphore_signal(semaphore);
    }];

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        SetMessage(result, @"Reminders proposed item cleanup timed out");
        return nil;
    }
    return matched;
}

static BOOL RemoveRemindersInProposedList(EKEventStore *store, MorrowEventKitCleanupResult *result) {
    EKCalendar *list = ContainerNamed(store, EKEntityTypeReminder, ProposedName);
    if (list == nil) {
        return YES;
    }

    NSArray<EKReminder *> *reminders = FetchReminders(store, list, result);
    if (reminders == nil) {
        return NO;
    }

    uint64_t removed = 0;
    for (EKReminder *reminder in reminders) {
        NSError *error = nil;
        if (![store removeReminder:reminder commit:NO error:&error]) {
            SetMessage(result, [NSString stringWithFormat:@"Reminders proposed item cleanup failed: %@", error.localizedDescription]);
            return NO;
        }
        removed += 1;
    }
    if (removed > 0) {
        NSError *error = nil;
        if (![store commit:&error]) {
            SetMessage(result, [NSString stringWithFormat:@"Reminders proposed item cleanup commit failed: %@", error.localizedDescription]);
            return NO;
        }
    }
    result->reminder_items_deleted = removed;
    return YES;
}

void morrow_eventkit_cleanup_proposed_items(MorrowEventKitCleanupResult *result) {
    @autoreleasepool {
        if (result == NULL) {
            return;
        }
        memset(result, 0, sizeof(*result));

        EKEventStore *store = [[EKEventStore alloc] init];
        if (!RequestAccess(store, EKEntityTypeEvent, result)) {
            return;
        }
        if (!RequestAccess(store, EKEntityTypeReminder, result)) {
            return;
        }

        result->calendar_items_deleted = RemoveEventsInProposedCalendar(store, result);
        if (result->message[0] != '\0') {
            return;
        }
        if (!RemoveRemindersInProposedList(store, result)) {
            return;
        }
        result->ok = 1;
    }
}
