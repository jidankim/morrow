#import "reminders_real_qa_support.h"

#import <dispatch/dispatch.h>

NSString *const ProposedListName = @"Morrow Proposed";
NSString *const ApprovalListName = @"Morrow Proposed QA Approved";
NSString *const DateOnlyTitle = @"Morrow QA TODO7 date-only synthetic";
NSString *const TimedTitle = @"Morrow QA TODO7 explicit-time synthetic";
NSString *const MoveTitle = @"Morrow QA TODO7 approval-move synthetic";
NSString *const DeleteTitle = @"Morrow QA F3 delete-reject synthetic";
NSString *const CopySourceTitle = @"Morrow QA F3 copy-source synthetic";
NSString *const CopyTargetTitle = @"Morrow QA F3 copy-target synthetic";
static NSString *const QAMarker = @"MorrowRealQA:TODO7 synthetic only";

NSError *QAError(NSString *message) {
    return [NSError errorWithDomain:@"MorrowRemindersRealQA"
                               code:1
                           userInfo:@{NSLocalizedDescriptionKey: message}];
}

NSString *StatusName(EKAuthorizationStatus status) {
    switch (status) {
        case EKAuthorizationStatusNotDetermined:
            return @"not_determined";
        case EKAuthorizationStatusRestricted:
            return @"restricted";
        case EKAuthorizationStatusDenied:
            return @"denied";
        case EKAuthorizationStatusFullAccess:
            return @"full_access";
        case EKAuthorizationStatusWriteOnly:
            return @"write_only";
        default:
            return [NSString stringWithFormat:@"unknown_%ld", (long)status];
    }
}

BOOL HasReadWriteAccess(EKAuthorizationStatus status) {
    return status == EKAuthorizationStatusFullAccess;
}

void PrintBlocked(NSString *reason, NSString *action) {
    printf("scenario=real-reminders-eventkit\n");
    printf("status=blocked\n");
    printf("reason=%s\n", [reason UTF8String]);
    printf("required_action=%s\n", [action UTF8String]);
}

void PrintFailure(NSString *reason, NSError *error) {
    printf("scenario=real-reminders-eventkit\n");
    printf("status=failed\n");
    printf("reason=%s\n", [reason UTF8String]);
    if (error != nil) {
        printf("error=%s\n", [[error localizedDescription] UTF8String]);
    }
}

EKCalendar *FindReminderCalendar(EKEventStore *store, NSString *title) {
    for (EKCalendar *calendar in [store calendarsForEntityType:EKEntityTypeReminder]) {
        if ([calendar.title isEqualToString:title]) {
            return calendar;
        }
    }
    return nil;
}

NSArray<EKReminder *> *FetchMarkedReminders(
    EKEventStore *store,
    EKCalendar *calendar,
    NSError **error) {
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    NSPredicate *predicate = [store predicateForRemindersInCalendars:@[calendar]];
    __block NSArray<EKReminder *> *matched = @[];

    [store fetchRemindersMatchingPredicate:predicate
                                completion:^(NSArray<EKReminder *> *reminders) {
        NSMutableArray<EKReminder *> *filtered = [NSMutableArray array];
        for (EKReminder *reminder in reminders) {
            if ([reminder.notes isEqualToString:QAMarker]) {
                [filtered addObject:reminder];
            }
        }
        matched = [filtered copy];
        dispatch_semaphore_signal(semaphore);
    }];

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 10LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        if (error != NULL) {
            *error = QAError(@"timed out reading marked reminders");
        }
        return nil;
    }
    return matched;
}

EKReminder *FindMarkedReminder(NSArray<EKReminder *> *reminders, NSString *title) {
    for (EKReminder *reminder in reminders) {
        if ([reminder.title isEqualToString:title]) {
            return reminder;
        }
    }
    return nil;
}

NSDateComponents *DateOnlyComponents(void) {
    NSDateComponents *components = [[NSDateComponents alloc] init];
    components.calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
    components.year = 2026;
    components.month = 7;
    components.day = 17;
    return components;
}

NSDateComponents *TimedComponents(void) {
    NSDateComponents *components = [[NSDateComponents alloc] init];
    components.calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
    components.year = 2026;
    components.month = 7;
    components.day = 18;
    components.hour = 9;
    components.minute = 30;
    return components;
}

BOOL ComponentsMatchDate(
    NSDateComponents *components,
    NSInteger year,
    NSInteger month,
    NSInteger day) {
    return components.year == year && components.month == month && components.day == day;
}

NSUInteger CleanupMarkedReminders(EKEventStore *store, EKCalendar *calendar, NSError **error) {
    if (calendar == nil) {
        return 0;
    }
    NSArray<EKReminder *> *reminders = FetchMarkedReminders(store, calendar, error);
    if (reminders == nil) {
        return 0;
    }
    NSUInteger removed = 0;
    for (EKReminder *reminder in reminders) {
        if (![store removeReminder:reminder commit:NO error:error]) {
            return removed;
        }
        removed += 1;
    }
    if (removed > 0 && ![store commit:error]) {
        return removed;
    }
    return removed;
}

EKCalendar *CreateReminderCalendar(EKEventStore *store, NSString *title, NSError **error) {
    EKCalendar *defaultCalendar = [store defaultCalendarForNewReminders];
    if (defaultCalendar.source == nil) {
        if (error != NULL) {
            *error = QAError(@"default reminder source is unavailable");
        }
        return nil;
    }

    EKCalendar *calendar = [EKCalendar calendarForEntityType:EKEntityTypeReminder eventStore:store];
    calendar.title = title;
    calendar.source = defaultCalendar.source;
    if (![store saveCalendar:calendar commit:YES error:error]) {
        return nil;
    }
    return calendar;
}

BOOL SaveMarkedReminder(
    EKEventStore *store,
    EKCalendar *calendar,
    NSString *title,
    NSDateComponents *due,
    NSError **error) {
    EKReminder *reminder = [EKReminder reminderWithEventStore:store];
    reminder.calendar = calendar;
    reminder.title = title;
    reminder.notes = QAMarker;
    reminder.dueDateComponents = due;
    return [store saveReminder:reminder commit:YES error:error];
}

BOOL VerifyNoMarkedReminders(EKEventStore *store, EKCalendar *calendar, NSError **error) {
    if (calendar == nil) {
        return YES;
    }
    NSArray<EKReminder *> *remaining = FetchMarkedReminders(store, calendar, error);
    return remaining != nil && remaining.count == 0;
}

BOOL RequestAccessIfAsked(EKEventStore *store) {
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block BOOL granted = NO;

    if ([store respondsToSelector:@selector(requestFullAccessToRemindersWithCompletion:)]) {
        [store requestFullAccessToRemindersWithCompletion:^(BOOL accessGranted, NSError *error) {
            (void)error;
            granted = accessGranted;
            dispatch_semaphore_signal(semaphore);
        }];
    } else {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        [store requestAccessToEntityType:EKEntityTypeReminder
                              completion:^(BOOL accessGranted, NSError *error) {
            (void)error;
            granted = accessGranted;
            dispatch_semaphore_signal(semaphore);
        }];
#pragma clang diagnostic pop
    }

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 120LL * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(semaphore, timeout) != 0) {
        return NO;
    }
    return granted;
}
