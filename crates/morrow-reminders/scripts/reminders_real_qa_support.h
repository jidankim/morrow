#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

extern NSString *const ProposedListName;
extern NSString *const ApprovalListName;
extern NSString *const DateOnlyTitle;
extern NSString *const TimedTitle;
extern NSString *const MoveTitle;
extern NSString *const DeleteTitle;
extern NSString *const CopySourceTitle;
extern NSString *const CopyTargetTitle;

NSError *QAError(NSString *message);
NSString *StatusName(EKAuthorizationStatus status);
BOOL HasReadWriteAccess(EKAuthorizationStatus status);
void PrintBlocked(NSString *reason, NSString *action);
void PrintFailure(NSString *reason, NSError *error);
EKCalendar *FindReminderCalendar(EKEventStore *store, NSString *title);
NSArray<EKReminder *> *FetchMarkedReminders(
    EKEventStore *store,
    EKCalendar *calendar,
    NSError **error);
EKReminder *FindMarkedReminder(NSArray<EKReminder *> *reminders, NSString *title);
NSDateComponents *DateOnlyComponents(void);
NSDateComponents *TimedComponents(void);
BOOL ComponentsMatchDate(
    NSDateComponents *components,
    NSInteger year,
    NSInteger month,
    NSInteger day);
NSUInteger CleanupMarkedReminders(EKEventStore *store, EKCalendar *calendar, NSError **error);
EKCalendar *CreateReminderCalendar(EKEventStore *store, NSString *title, NSError **error);
BOOL SaveMarkedReminder(
    EKEventStore *store,
    EKCalendar *calendar,
    NSString *title,
    NSDateComponents *due,
    NSError **error);
BOOL VerifyNoMarkedReminders(EKEventStore *store, EKCalendar *calendar, NSError **error);
BOOL RequestAccessIfAsked(EKEventStore *store);
