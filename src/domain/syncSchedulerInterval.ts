import { z } from "zod"

export const MIN_SYNC_SCHEDULER_INTERVAL_MINUTES = 1
export const MIN_SYNC_SCHEDULER_INTERVAL_SECONDS = 60
export const DEFAULT_SYNC_SCHEDULER_INTERVAL_SECONDS = 1_800

export type SyncSchedulerIntervalSeconds = number

export const syncSchedulerIntervalSecondsSchema = z
  .number()
  .int()
  .min(MIN_SYNC_SCHEDULER_INTERVAL_SECONDS)
  .refine(
    (intervalSeconds) => intervalSeconds % MIN_SYNC_SCHEDULER_INTERVAL_SECONDS === 0,
    "Automatic sync interval must be whole minutes."
  )

const syncSchedulerIntervalMinutesSchema = z
  .number()
  .int()
  .min(MIN_SYNC_SCHEDULER_INTERVAL_MINUTES)

export function parseSyncSchedulerIntervalSeconds(value: unknown): SyncSchedulerIntervalSeconds {
  const parsed = syncSchedulerIntervalSecondsSchema.safeParse(value)
  if (parsed.success) return parsed.data
  throw new InvalidSyncSchedulerIntervalError(value)
}

export function parseSyncSchedulerIntervalMinutes(value: unknown): SyncSchedulerIntervalSeconds {
  const parsed = syncSchedulerIntervalMinutesSchema.safeParse(value)
  if (!parsed.success) throw new InvalidSyncSchedulerIntervalError(value)
  return parseSyncSchedulerIntervalSeconds(parsed.data * MIN_SYNC_SCHEDULER_INTERVAL_SECONDS)
}

export function syncSchedulerIntervalMinutes(intervalSeconds: SyncSchedulerIntervalSeconds): number {
  return intervalSeconds / MIN_SYNC_SCHEDULER_INTERVAL_SECONDS
}

export class InvalidSyncSchedulerIntervalError extends Error {
  readonly name = "InvalidSyncSchedulerIntervalError"
  constructor(readonly value: unknown) {
    super("Invalid sync scheduler interval")
  }
}
