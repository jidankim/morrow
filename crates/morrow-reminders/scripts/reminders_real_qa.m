#import "reminders_real_qa_support.h"

#import <Foundation/Foundation.h>
#import <string.h>

static int FailAfterCleanup(
    EKEventStore *store,
    EKCalendar *proposed,
    EKCalendar *approval,
    NSString *reason,
    NSError *error) {
    PrintFailure(reason, error);
    (void)CleanupMarkedReminders(store, proposed, nil);
    (void)CleanupMarkedReminders(store, approval, nil);
    return 1;
}

static BOOL EnsureCalendar(
    EKEventStore *store,
    NSString *name,
    EKCalendar **calendar,
    BOOL *created,
    NSError **error) {
    *calendar = FindReminderCalendar(store, name);
    *created = NO;
    if (*calendar != nil) {
        return YES;
    }
    *calendar = CreateReminderCalendar(store, name, error);
    *created = *calendar != nil;
    return *calendar != nil;
}

static BOOL RemoveCreatedCalendar(
    EKEventStore *store,
    EKCalendar *calendar,
    BOOL created,
    NSString *reason) {
    if (!created) {
        return YES;
    }
    NSError *error = nil;
    if (![store removeCalendar:calendar commit:YES error:&error]) {
        PrintFailure(reason, error);
        return NO;
    }
    return YES;
}

static int RequestOrBlock(EKEventStore *store, BOOL requestAccess) {
    NSString *command = @"crates/morrow-reminders/scripts/reminders_real_qa.sh";
    if (requestAccess) {
        BOOL granted = RequestAccessIfAsked(store);
        PrintBlocked(
            @"permission_prompt_completed",
            granted
                ? [NSString stringWithFormat:@"Reminders access was granted; rerun %@", command]
                : [NSString stringWithFormat:@"Grant Reminders access when prompted, then rerun %@", command]);
        return 77;
    }
    PrintBlocked(
        @"reminders_access_not_determined",
        [NSString stringWithFormat:@"Run %@ --request-access in an interactive macOS Terminal, approve Reminders access for the generated QA binary, then rerun %@", command, command]);
    return 77;
}

static int VerifyRealReminders(EKEventStore *store) {
    NSError *error = nil;
    EKCalendar *proposed = nil;
    EKCalendar *approval = nil;
    BOOL proposedCreated = NO;
    BOOL approvalCreated = NO;
    BOOL proposedPreexisting = FindReminderCalendar(store, ProposedListName) != nil;
    BOOL approvalPreexisting = FindReminderCalendar(store, ApprovalListName) != nil;

    if (!EnsureCalendar(store, ProposedListName, &proposed, &proposedCreated, &error)) {
        PrintFailure(@"create_proposed_list_failed", error);
        return 1;
    }
    if (!EnsureCalendar(store, ApprovalListName, &approval, &approvalCreated, &error)) {
        PrintFailure(@"create_approval_list_failed", error);
        return 1;
    }

    NSUInteger staleProposedRemoved = CleanupMarkedReminders(store, proposed, &error);
    if (error != nil) {
        return FailAfterCleanup(store, proposed, approval, @"stale_proposed_cleanup_failed", error);
    }
    NSUInteger staleApprovalRemoved = CleanupMarkedReminders(store, approval, &error);
    if (error != nil) {
        return FailAfterCleanup(store, proposed, approval, @"stale_approval_cleanup_failed", error);
    }

    BOOL createdReminders = SaveMarkedReminder(store, proposed, DateOnlyTitle, DateOnlyComponents(), &error)
        && SaveMarkedReminder(store, proposed, TimedTitle, TimedComponents(), &error)
        && SaveMarkedReminder(store, proposed, MoveTitle, TimedComponents(), &error)
        && SaveMarkedReminder(store, proposed, DeleteTitle, TimedComponents(), &error)
        && SaveMarkedReminder(store, proposed, CopySourceTitle, TimedComponents(), &error);
    if (!createdReminders) {
        return FailAfterCleanup(store, proposed, approval, @"create_reminders_failed", error);
    }

    NSArray<EKReminder *> *readback = FetchMarkedReminders(store, proposed, &error);
    EKReminder *dateOnly = FindMarkedReminder(readback, DateOnlyTitle);
    EKReminder *timed = FindMarkedReminder(readback, TimedTitle);
    EKReminder *move = FindMarkedReminder(readback, MoveTitle);
    EKReminder *deleteCandidate = FindMarkedReminder(readback, DeleteTitle);
    EKReminder *copySource = FindMarkedReminder(readback, CopySourceTitle);
    if (dateOnly == nil || timed == nil || move == nil || deleteCandidate == nil || copySource == nil) {
        return FailAfterCleanup(store, proposed, approval, @"readback_missing_created_reminders", nil);
    }

    NSDateComponents *dateOnlyDue = dateOnly.dueDateComponents;
    BOOL dateOnlyHasNoTime = dateOnlyDue.hour == NSDateComponentUndefined
        && dateOnlyDue.minute == NSDateComponentUndefined
        && dateOnlyDue.second == NSDateComponentUndefined;
    if (!ComponentsMatchDate(dateOnlyDue, 2026, 7, 17) || !dateOnlyHasNoTime) {
        return FailAfterCleanup(store, proposed, approval, @"date_only_due_time_was_invented", nil);
    }

    NSDateComponents *timedDue = timed.dueDateComponents;
    if (!ComponentsMatchDate(timedDue, 2026, 7, 18) || timedDue.hour != 9 || timedDue.minute != 30) {
        return FailAfterCleanup(store, proposed, approval, @"explicit_due_time_readback_mismatch", nil);
    }

    dateOnly.completed = YES;
    if (![store saveReminder:dateOnly commit:YES error:&error]) {
        return FailAfterCleanup(store, proposed, approval, @"completion_save_failed", error);
    }
    NSArray<EKReminder *> *completionReadback = FetchMarkedReminders(store, proposed, &error);
    EKReminder *completed = FindMarkedReminder(completionReadback, DateOnlyTitle);
    if (completed == nil || !completed.completed) {
        return FailAfterCleanup(store, proposed, approval, @"completion_readback_mismatch", nil);
    }

    move.calendar = approval;
    if (![store saveReminder:move commit:YES error:&error]) {
        return FailAfterCleanup(store, proposed, approval, @"move_save_failed", error);
    }
    NSArray<EKReminder *> *approvalReadback = FetchMarkedReminders(store, approval, &error);
    EKReminder *moved = FindMarkedReminder(approvalReadback, MoveTitle);
    if (moved == nil || ![moved.calendar.calendarIdentifier isEqualToString:approval.calendarIdentifier]) {
        return FailAfterCleanup(store, proposed, approval, @"move_readback_mismatch", nil);
    }

    if (![store removeReminder:deleteCandidate commit:YES error:&error]) {
        return FailAfterCleanup(store, proposed, approval, @"delete_rejection_save_failed", error);
    }
    NSArray<EKReminder *> *deleteReadback = FetchMarkedReminders(store, proposed, &error);
    if (FindMarkedReminder(deleteReadback, DeleteTitle) != nil) {
        return FailAfterCleanup(store, proposed, approval, @"delete_rejection_readback_still_present", nil);
    }

    EKReminder *copyTarget = [EKReminder reminderWithEventStore:store];
    copyTarget.calendar = approval;
    copyTarget.title = CopyTargetTitle;
    copyTarget.notes = copySource.notes;
    copyTarget.dueDateComponents = copySource.dueDateComponents;
    if (![store saveReminder:copyTarget commit:YES error:&error]) {
        return FailAfterCleanup(store, proposed, approval, @"copy_target_save_failed", error);
    }
    if (![store removeReminder:copySource commit:YES error:&error]) {
        return FailAfterCleanup(store, proposed, approval, @"copy_source_cleanup_failed", error);
    }
    NSArray<EKReminder *> *copyProposedReadback = FetchMarkedReminders(store, proposed, &error);
    NSArray<EKReminder *> *copyApprovalReadback = FetchMarkedReminders(store, approval, &error);
    if (FindMarkedReminder(copyProposedReadback, CopySourceTitle) != nil
        || FindMarkedReminder(copyApprovalReadback, CopyTargetTitle) == nil) {
        return FailAfterCleanup(store, proposed, approval, @"copy_readback_mismatch", nil);
    }

    NSUInteger cleanupProposedRemoved = CleanupMarkedReminders(store, proposed, &error);
    NSUInteger cleanupApprovalRemoved = CleanupMarkedReminders(store, approval, &error);
    if (error != nil || !VerifyNoMarkedReminders(store, proposed, &error)
        || !VerifyNoMarkedReminders(store, approval, &error)) {
        return FailAfterCleanup(store, proposed, approval, @"cleanup_readback_not_empty", error);
    }
    if (!RemoveCreatedCalendar(store, approval, approvalCreated, @"approval_list_cleanup_failed")
        || !RemoveCreatedCalendar(store, proposed, proposedCreated, @"proposed_list_cleanup_failed")) {
        return 1;
    }

    printf("scenario=real-reminders-eventkit\nstatus=passed\n");
    printf("proposed_list=Morrow Proposed\n");
    printf("preexisting_proposed=%s\n", proposedPreexisting ? "true" : "false");
    printf("preexisting_approval_list=%s\n", approvalPreexisting ? "true" : "false");
    printf("stale_test_items_removed=%lu\n", (unsigned long)(staleProposedRemoved + staleApprovalRemoved));
    printf("date_only_readback=2026-07-17 time=absent\n");
    printf("explicit_time_readback=2026-07-18 09:30\n");
    printf("completion_observation=rejected_resolved_in_proposed\n");
    printf("move_observation=approved_by_move_to_controlled_list\n");
    printf("delete_observation=rejected_by_delete_from_proposed\n");
    printf("copy_observation=approved_by_copy_to_controlled_list_source_proposal_removed\n");
    printf("cleanup_test_items_removed=%lu\n", (unsigned long)(cleanupProposedRemoved + cleanupApprovalRemoved));
    printf("cleanup_test_items_remaining=0\n");
    printf("cleanup_created_proposed_list=%s\n", proposedCreated ? "true" : "false");
    printf("cleanup_created_approval_list=%s\n", approvalCreated ? "true" : "false");
    printf("PASS reminders_real_qa\n");
    return 0;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        BOOL requestAccess = argc == 2 && strcmp(argv[1], "--request-access") == 0;
        EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
        EKEventStore *store = [[EKEventStore alloc] init];
        if (status == EKAuthorizationStatusNotDetermined) {
            return RequestOrBlock(store, requestAccess);
        }
        if (!HasReadWriteAccess(status)) {
            PrintBlocked(
                [NSString stringWithFormat:@"reminders_access_%@", StatusName(status)],
                @"Enable Reminders access for this command-line QA binary in System Settings > Privacy & Security > Reminders, then rerun crates/morrow-reminders/scripts/reminders_real_qa.sh");
            return 77;
        }
        return VerifyRealReminders(store);
    }
}
