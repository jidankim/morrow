import { z } from "zod"
import { SYSTEM_REFERENCE_TIME_ZONE, isReferenceTimeZonePreference } from "./timeZone"

export type CalendarSource = "apple-calendar"

export type ListReminderProfile = {
  readonly enabled: boolean
  readonly profileId: "list-reminders"
  readonly profileVersion: "list-reminders-v1"
  readonly routingMode: "explicitOnly" | "profileBareQuantityLists"
  readonly defaultDueMode: "explicitOnly" | "nextLocalDayAtDefaultTime"
  readonly defaultDueTime: "23:59"
  readonly recurrenceMode: "none"
  readonly itemOutputMode: "singleReminderTitle"
}

export type AppConfig = {
  readonly referenceTimezone: string
  readonly calendarSource: CalendarSource
  readonly permissionsGranted: boolean
  readonly launchAtLogin: boolean
  readonly sourceExcerptsEnabled: boolean
  readonly feedbackTextSnapshotsEnabled: boolean
  readonly firstProposalGuidanceEnabled: boolean
  readonly telemetryEnabled: false
  readonly crashLogExcerptsEnabled: false
  readonly localDiagnosticsEnabled: boolean
  readonly localDiagnosticsRetentionDays: number
  readonly listReminderProfile: ListReminderProfile
}

const calendarSourceSchema = z.literal("apple-calendar")
export const DEFAULT_LIST_REMINDER_PROFILE = {
  enabled: false,
  profileId: "list-reminders",
  profileVersion: "list-reminders-v1",
  routingMode: "explicitOnly",
  defaultDueMode: "explicitOnly",
  defaultDueTime: "23:59",
  recurrenceMode: "none",
  itemOutputMode: "singleReminderTitle"
} as const satisfies ListReminderProfile

const timeZoneSchema = z.string().refine((value) => isReferenceTimeZonePreference(value), {
  message: "Reference timezone must be supported by Morrow Calendar replay."
})

const listReminderProfileSchema = z.object({
  enabled: z.boolean().default(DEFAULT_LIST_REMINDER_PROFILE.enabled),
  profileId: z.literal(DEFAULT_LIST_REMINDER_PROFILE.profileId).default(DEFAULT_LIST_REMINDER_PROFILE.profileId),
  profileVersion: z
    .literal(DEFAULT_LIST_REMINDER_PROFILE.profileVersion)
    .default(DEFAULT_LIST_REMINDER_PROFILE.profileVersion),
  routingMode: z
    .union([z.literal("explicitOnly"), z.literal("profileBareQuantityLists")])
    .default(DEFAULT_LIST_REMINDER_PROFILE.routingMode),
  defaultDueMode: z
    .union([z.literal("explicitOnly"), z.literal("nextLocalDayAtDefaultTime")])
    .default(DEFAULT_LIST_REMINDER_PROFILE.defaultDueMode),
  defaultDueTime: z
    .literal(DEFAULT_LIST_REMINDER_PROFILE.defaultDueTime)
    .default(DEFAULT_LIST_REMINDER_PROFILE.defaultDueTime),
  recurrenceMode: z.literal(DEFAULT_LIST_REMINDER_PROFILE.recurrenceMode).default(DEFAULT_LIST_REMINDER_PROFILE.recurrenceMode),
  itemOutputMode: z
    .literal(DEFAULT_LIST_REMINDER_PROFILE.itemOutputMode)
    .default(DEFAULT_LIST_REMINDER_PROFILE.itemOutputMode)
})

export const appConfigSchema = z.object({
  referenceTimezone: timeZoneSchema,
  calendarSource: calendarSourceSchema,
  permissionsGranted: z.boolean(),
  launchAtLogin: z.boolean(),
  sourceExcerptsEnabled: z.boolean(),
  feedbackTextSnapshotsEnabled: z.boolean().default(false),
  firstProposalGuidanceEnabled: z.boolean(),
  telemetryEnabled: z.literal(false).default(false),
  crashLogExcerptsEnabled: z.literal(false).default(false),
  localDiagnosticsEnabled: z.boolean().default(false),
  localDiagnosticsRetentionDays: z.number().int().min(1).max(365).default(30),
  listReminderProfile: listReminderProfileSchema.default(DEFAULT_LIST_REMINDER_PROFILE)
})

export function createDefaultAppConfig(browserTimeZone: string): AppConfig {
  return {
    referenceTimezone: SYSTEM_REFERENCE_TIME_ZONE,
    calendarSource: "apple-calendar",
    permissionsGranted: false,
    launchAtLogin: false,
    sourceExcerptsEnabled: true,
    feedbackTextSnapshotsEnabled: false,
    firstProposalGuidanceEnabled: true,
    telemetryEnabled: false,
    crashLogExcerptsEnabled: false,
    localDiagnosticsEnabled: false,
    localDiagnosticsRetentionDays: 30,
    listReminderProfile: { ...DEFAULT_LIST_REMINDER_PROFILE }
  }
}
