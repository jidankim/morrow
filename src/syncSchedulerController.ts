import {
  getSyncReadinessItems,
  type AppShellState,
  type NativeAppShellState
} from "./domain/appShell"
import {
  createDefaultSyncSchedulerState,
  formatSyncSchedulerNextRun,
  recoverHydratedSyncSchedulerState,
  type SyncSchedulerState
} from "./domain/syncScheduler"

export type HydratedSyncSchedulerState = {
  readonly state: SyncSchedulerState
  readonly shouldPersist: boolean
}

export type SchedulerShellStateFields = Pick<
  NativeAppShellState,
  "automaticSyncEnabled" | "automaticSyncStatusLabel" | "automaticSyncDetail"
>

export function currentUnixSeconds(): number {
  return Math.floor(Date.now() / 1_000)
}

export function hydrateSyncSchedulerState(
  persistedState: SyncSchedulerState | undefined,
  nowUnixSeconds: number
): HydratedSyncSchedulerState {
  if (persistedState === undefined) {
    return {
      state: createDefaultSyncSchedulerState(nowUnixSeconds),
      shouldPersist: false
    }
  }
  const recovered = recoverHydratedSyncSchedulerState(persistedState, nowUnixSeconds)
  return {
    state: recovered,
    shouldPersist: persistedState.status === "running"
  }
}

export function syncSchedulerShellStateFields(
  state: SyncSchedulerState,
  nowUnixSeconds: number
): SchedulerShellStateFields {
  switch (state.status) {
    case "disabled":
      return {
        automaticSyncEnabled: false,
        automaticSyncStatusLabel: "Off",
        automaticSyncDetail: formatSyncSchedulerNextRun(state, nowUnixSeconds)
      }
    case "scheduled":
    case "running":
      return {
        automaticSyncEnabled: state.enabled,
        automaticSyncStatusLabel: "On",
        automaticSyncDetail: formatSyncSchedulerNextRun(state, nowUnixSeconds)
      }
    case "cooldown":
      return {
        automaticSyncEnabled: state.enabled,
        automaticSyncStatusLabel: "Cooling Down",
        automaticSyncDetail: formatSyncSchedulerNextRun(state, nowUnixSeconds)
      }
    case "blocked":
      return {
        automaticSyncEnabled: state.enabled,
        automaticSyncStatusLabel: "Needs Action",
        automaticSyncDetail: state.last_reason ?? "Automatic sync needs action."
      }
    default:
      return assertNever(state.status)
  }
}

export function nextSyncSchedulerWakeDelayMs(
  state: SyncSchedulerState,
  nowUnixSeconds: number
): number | undefined {
  const wakeTimestamp = nextSyncSchedulerWakeTimestamp(state)
  return wakeTimestamp === undefined ? undefined : Math.max(0, (wakeTimestamp - nowUnixSeconds) * 1_000)
}

export function automaticSyncBlockedReason(state: AppShellState): string | undefined {
  return getSyncReadinessItems(state, { syncing: false }).find(
    (item) => item.id !== "sync-activity" && item.status === "blocking"
  )?.detail
}

function nextSyncSchedulerWakeTimestamp(state: SyncSchedulerState): number | undefined {
  switch (state.status) {
    case "scheduled":
      return state.next_run_at
    case "cooldown":
      return state.next_eligible_at
    case "disabled":
    case "running":
    case "blocked":
      return undefined
    default:
      return assertNever(state.status)
  }
}

function assertNever(value: never): never {
  throw new UnhandledSyncSchedulerControllerVariantError(String(value))
}

class UnhandledSyncSchedulerControllerVariantError extends Error {
  readonly name = "UnhandledSyncSchedulerControllerVariantError"
  constructor(readonly renderedValue: string) {
    super(`Unhandled sync scheduler controller variant: ${renderedValue}`)
  }
}
