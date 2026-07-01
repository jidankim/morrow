import { z } from "zod"

export const SYNC_SCHEDULER_INTERVAL_OPTIONS = [900, 1_800, 3_600] as const

export type SyncSchedulerIntervalSeconds = (typeof SYNC_SCHEDULER_INTERVAL_OPTIONS)[number]
export type SyncSchedulerStatus = "disabled" | "scheduled" | "running" | "cooldown" | "blocked"
export type SyncSchedulerLastResult = "success" | "retryable_failure" | "blocked" | "manual_disabled"

export type SyncSchedulerState = {
  readonly enabled: boolean
  readonly interval_seconds: SyncSchedulerIntervalSeconds
  readonly status: SyncSchedulerStatus
  readonly last_started_at?: number | undefined
  readonly last_finished_at?: number | undefined
  readonly next_run_at?: number | undefined
  readonly next_eligible_at?: number | undefined
  readonly last_result?: SyncSchedulerLastResult | undefined
  readonly retry_attempt: number
  readonly last_reason?: string | undefined
  readonly updated_at: number
}

type TimestampedTransition = { readonly nowUnixSeconds: number }
type IntervalTransition = TimestampedTransition & { readonly intervalSeconds: SyncSchedulerIntervalSeconds }
type ReasonTransition = TimestampedTransition & { readonly reason: string }
type RetryableFailureTransition = ReasonTransition & { readonly randomUnit: number }

const syncSchedulerIntervalSchema = z.union([z.literal(900), z.literal(1_800), z.literal(3_600)])
const syncSchedulerStatusSchema = z.union([z.literal("disabled"), z.literal("scheduled"), z.literal("running"), z.literal("cooldown"), z.literal("blocked")])
const syncSchedulerLastResultSchema = z.union([z.literal("success"), z.literal("retryable_failure"), z.literal("blocked"), z.literal("manual_disabled")])
const syncSchedulerStateSchema = z.object({
  enabled: z.boolean(),
  interval_seconds: syncSchedulerIntervalSchema,
  status: syncSchedulerStatusSchema,
  last_started_at: z.number().int().nonnegative().optional(),
  last_finished_at: z.number().int().nonnegative().optional(),
  next_run_at: z.number().int().nonnegative().optional(),
  next_eligible_at: z.number().int().nonnegative().optional(),
  last_result: syncSchedulerLastResultSchema.optional(),
  retry_attempt: z.number().int().min(0),
  last_reason: z.string().min(1).optional(),
  updated_at: z.number().int().nonnegative()
})

export function createDefaultSyncSchedulerState(nowUnixSeconds: number): SyncSchedulerState {
  return { enabled: false, interval_seconds: 1_800, status: "disabled", retry_attempt: 0, updated_at: nowUnixSeconds }
}

export function parseSyncSchedulerIntervalSeconds(value: unknown): SyncSchedulerIntervalSeconds {
  const parsed = syncSchedulerIntervalSchema.safeParse(value)
  if (parsed.success) return parsed.data
  throw new InvalidSyncSchedulerIntervalError(value)
}

export function parseSyncSchedulerState(value: unknown): SyncSchedulerState {
  const parsed = syncSchedulerStateSchema.safeParse(value)
  if (parsed.success) return parsed.data
  throw new InvalidSyncSchedulerStateError(parsed.error.message)
}

export function enableSyncScheduler(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  return scheduleNextRun(state, nowUnixSeconds, state.interval_seconds)
}

export function disableSyncScheduler(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  return {
    ...state,
    enabled: false,
    status: "disabled",
    next_run_at: undefined,
    next_eligible_at: undefined,
    last_result: "manual_disabled",
    retry_attempt: 0,
    last_reason: undefined,
    updated_at: nowUnixSeconds
  }
}

export function changeSyncSchedulerInterval(state: SyncSchedulerState, transition: IntervalTransition): SyncSchedulerState {
  if (state.enabled) return scheduleNextRun(state, transition.nowUnixSeconds, transition.intervalSeconds)
  return { ...disableSyncScheduler(state, transition.nowUnixSeconds), interval_seconds: transition.intervalSeconds }
}

export function markSyncSchedulerRunning(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  return {
    ...state,
    enabled: true,
    status: "running",
    last_started_at: nowUnixSeconds,
    next_run_at: undefined,
    next_eligible_at: undefined,
    last_reason: undefined,
    updated_at: nowUnixSeconds
  }
}

export function markSyncSchedulerSuccess(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  return { ...scheduleNextRun(state, nowUnixSeconds, state.interval_seconds), last_finished_at: nowUnixSeconds, last_result: "success" }
}

export function markSyncSchedulerBlocked(state: SyncSchedulerState, transition: ReasonTransition): SyncSchedulerState {
  return {
    ...state,
    enabled: true,
    status: "blocked",
    next_run_at: undefined,
    next_eligible_at: undefined,
    last_result: "blocked",
    last_reason: transition.reason,
    updated_at: transition.nowUnixSeconds
  }
}

export function markSyncSchedulerRetryableFailure(
  state: SyncSchedulerState,
  transition: RetryableFailureTransition
): SyncSchedulerState {
  const retryAttempt = state.retry_attempt + 1
  return {
    ...state,
    enabled: true,
    status: "cooldown",
    next_run_at: undefined,
    next_eligible_at: transition.nowUnixSeconds + calculateRetryDelaySeconds(retryAttempt, transition.randomUnit),
    last_result: "retryable_failure",
    retry_attempt: retryAttempt,
    last_reason: transition.reason,
    updated_at: transition.nowUnixSeconds
  }
}

export function markSyncSchedulerInFlightOverlap(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  return {
    ...state,
    enabled: true,
    status: "scheduled",
    next_run_at: nowUnixSeconds + 60,
    next_eligible_at: undefined,
    last_reason: "Waiting for current Sync Now to finish.",
    updated_at: nowUnixSeconds
  }
}

export function recoverHydratedSyncSchedulerState(state: SyncSchedulerState, nowUnixSeconds: number): SyncSchedulerState {
  switch (state.status) {
    case "running":
      return {
        ...state,
        status: "cooldown",
        next_run_at: undefined,
        next_eligible_at: calculateRestartCooldownTimestamp(state, nowUnixSeconds),
        retry_attempt: Math.max(1, state.retry_attempt),
        last_result: "retryable_failure",
        last_reason: "Morrow restarted before automatic sync finished.",
        updated_at: nowUnixSeconds
      }
    case "disabled":
    case "scheduled":
    case "cooldown":
    case "blocked":
      return state
    default:
      return assertNever(state.status)
  }
}

export function isSyncSchedulerDue(state: SyncSchedulerState, nowUnixSeconds: number): boolean {
  if (!state.enabled) return false
  switch (state.status) {
    case "scheduled":
      return state.next_run_at !== undefined && state.next_run_at <= nowUnixSeconds
    case "cooldown":
      return state.next_eligible_at !== undefined && state.next_eligible_at <= nowUnixSeconds
    case "disabled":
    case "running":
    case "blocked":
      return false
    default:
      return assertNever(state.status)
  }
}

export function formatSyncSchedulerNextRun(state: SyncSchedulerState, nowUnixSeconds: number): string {
  switch (state.status) {
    case "disabled":
      return "Automatic sync is off."
    case "scheduled":
      return formatFutureTimestamp("Next automatic sync", state.next_run_at, nowUnixSeconds)
    case "cooldown":
      return formatFutureTimestamp("Retry automatic sync", state.next_eligible_at, nowUnixSeconds)
    case "running":
      return "Automatic sync is running."
    case "blocked":
      return "Automatic sync needs action."
    default:
      return assertNever(state.status)
  }
}

export function formatSyncSchedulerLastResult(state: SyncSchedulerState): string {
  if (state.last_result === undefined) return "No automatic sync has run yet."
  switch (state.last_result) {
    case "success":
      return "Last automatic sync succeeded."
    case "retryable_failure":
      return "Last automatic sync will retry."
    case "blocked":
      return "Last automatic sync needs action."
    case "manual_disabled":
      return "Automatic sync was turned off."
    default:
      return assertNever(state.last_result)
  }
}

export function formatSyncSchedulerCooldownReason(state: SyncSchedulerState): string {
  return state.last_reason ?? "Automatic sync is waiting before retrying."
}

function scheduleNextRun(state: SyncSchedulerState, nowUnixSeconds: number, intervalSeconds: SyncSchedulerIntervalSeconds): SyncSchedulerState {
  return {
    ...state,
    enabled: true,
    interval_seconds: intervalSeconds,
    status: "scheduled",
    next_run_at: nowUnixSeconds + intervalSeconds,
    next_eligible_at: undefined,
    retry_attempt: 0,
    last_reason: undefined,
    updated_at: nowUnixSeconds
  }
}

function calculateRetryDelaySeconds(retryAttempt: number, randomUnit: number): number {
  const cappedExponential = Math.min(1_800, 60 * 2 ** Math.min(retryAttempt - 1, 5))
  return Math.min(1_800, Math.round(cappedExponential * (0.8 + randomUnit * 0.4)))
}

function calculateRestartCooldownTimestamp(state: SyncSchedulerState, nowUnixSeconds: number): number {
  const fallbackTimestamp = nowUnixSeconds + 60
  return state.last_started_at === undefined ? fallbackTimestamp : Math.max(fallbackTimestamp, state.last_started_at + 600)
}

function formatFutureTimestamp(label: "Next automatic sync" | "Retry automatic sync", unixSeconds: number | undefined, nowUnixSeconds: number): string {
  if (unixSeconds === undefined || unixSeconds <= nowUnixSeconds) return `${label} now.`
  return `${label} in ${formatDurationSeconds(unixSeconds - nowUnixSeconds)}.`
}

function formatDurationSeconds(seconds: number): string {
  if (seconds < 60) return `${seconds} sec`
  return `${Math.round(seconds / 60)} min`
}

function assertNever(value: never): never {
  throw new UnhandledSyncSchedulerVariantError(String(value))
}

export class InvalidSyncSchedulerIntervalError extends Error {
  readonly name = "InvalidSyncSchedulerIntervalError"
  constructor(readonly value: unknown) {
    super("Invalid sync scheduler interval")
  }
}

export class InvalidSyncSchedulerStateError extends Error {
  readonly name = "InvalidSyncSchedulerStateError"
  constructor(readonly validationMessage: string) {
    super("Invalid sync scheduler state")
  }
}

class UnhandledSyncSchedulerVariantError extends Error {
  readonly name = "UnhandledSyncSchedulerVariantError"
  constructor(readonly renderedValue: string) {
    super(`Unhandled sync scheduler variant: ${renderedValue}`)
  }
}
