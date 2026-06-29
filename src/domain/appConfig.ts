import { z } from "zod"
import { isSupportedReferenceTimeZone, normalizeReferenceTimeZone } from "./timeZone"

export type CalendarSource = "apple-calendar"

export type AppConfig = {
  readonly referenceTimezone: string
  readonly calendarSource: CalendarSource
  readonly permissionsGranted: boolean
  readonly launchAtLogin: boolean
  readonly sourceExcerptsEnabled: boolean
  readonly firstProposalGuidanceEnabled: boolean
  readonly telemetryEnabled: false
  readonly crashLogExcerptsEnabled: false
  readonly localDiagnosticsEnabled: boolean
  readonly localDiagnosticsRetentionDays: number
}

const calendarSourceSchema = z.literal("apple-calendar")

const timeZoneSchema = z.string().refine((value) => isSupportedReferenceTimeZone(value), {
  message: "Reference timezone must be supported by Morrow Calendar replay."
})

export const appConfigSchema = z.object({
  referenceTimezone: timeZoneSchema,
  calendarSource: calendarSourceSchema,
  permissionsGranted: z.boolean(),
  launchAtLogin: z.boolean(),
  sourceExcerptsEnabled: z.boolean(),
  firstProposalGuidanceEnabled: z.boolean(),
  telemetryEnabled: z.literal(false).default(false),
  crashLogExcerptsEnabled: z.literal(false).default(false),
  localDiagnosticsEnabled: z.boolean().default(false),
  localDiagnosticsRetentionDays: z.number().int().min(1).max(365).default(30)
})

export function createDefaultAppConfig(browserTimeZone: string): AppConfig {
  return {
    referenceTimezone: normalizeReferenceTimeZone(browserTimeZone),
    calendarSource: "apple-calendar",
    permissionsGranted: false,
    launchAtLogin: false,
    sourceExcerptsEnabled: true,
    firstProposalGuidanceEnabled: true,
    telemetryEnabled: false,
    crashLogExcerptsEnabled: false,
    localDiagnosticsEnabled: false,
    localDiagnosticsRetentionDays: 30
  }
}
