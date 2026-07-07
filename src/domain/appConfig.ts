import { z } from "zod"
import { SYSTEM_REFERENCE_TIME_ZONE, isReferenceTimeZonePreference } from "./timeZone"
import {
  DEFAULT_LIST_INTAKE_PROFILES,
  legacyListReminderProfileSchema,
  listIntakeProfilesSchema,
  migrateLegacyListReminderProfile,
  type ListIntakeProfile
} from "./listIntakeProfile"

export type CalendarSource = "apple-calendar"

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
  readonly listIntakeProfiles: readonly ListIntakeProfile[]
}

const calendarSourceSchema = z.literal("apple-calendar")

const timeZoneSchema = z.string().refine((value) => isReferenceTimeZonePreference(value), {
  message: "Reference timezone must be supported by Morrow Calendar replay."
})

export const appConfigSchema = z
  .object({
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
    listIntakeProfiles: listIntakeProfilesSchema.optional(),
    listReminderProfile: legacyListReminderProfileSchema.optional()
  })
  .transform((config) => {
    const { listIntakeProfiles, listReminderProfile, ...currentConfig } = config
    return {
      ...currentConfig,
      listIntakeProfiles:
        listIntakeProfiles ?? migrateLegacyListReminderProfile(listReminderProfile)
    }
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
    listIntakeProfiles: DEFAULT_LIST_INTAKE_PROFILES
  }
}

export type {
  ListIntakeCategoryRule,
  ListIntakeChatScope,
  ListIntakeDigestReminder,
  ListIntakeOutputPolicy,
  ListIntakeProfile
} from "./listIntakeProfile"
export {
  DEFAULT_LEGACY_LIST_REMINDER_PROFILE,
  createListIntakeExamplesHash,
  createListIntakeProfile,
  listIntakeProfilesSchema
} from "./listIntakeProfile"
