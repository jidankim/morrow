import { useEffect, type MutableRefObject } from "react"
import { getMenuModel, isSyncNowEnabled, type AppShellState } from "./domain/appShell"
import {
  isSyncSchedulerDue,
  markSyncSchedulerBlocked,
  markSyncSchedulerInFlightOverlap,
  type SyncSchedulerState
} from "./domain/syncScheduler"
import {
  automaticSyncBlockedReason,
  currentUnixSeconds,
  nextSyncSchedulerWakeDelayMs
} from "./syncSchedulerController"

type PersistSyncSchedulerState = (
  nextState: SyncSchedulerState
) => Promise<SyncSchedulerState>

type SyncSchedulerTimerOptions = {
  readonly executeAutomaticSync: () => void
  readonly persistSyncSchedulerState: PersistSyncSchedulerState
  readonly reportSyncSchedulerPersistenceFailure: (error: unknown) => void
  readonly stateRef: MutableRefObject<AppShellState>
  readonly syncInFlight: MutableRefObject<boolean>
  readonly syncScheduler: SyncSchedulerState
  readonly syncSchedulerHydrated: boolean
  readonly syncSchedulerRef: MutableRefObject<SyncSchedulerState>
}

export function useSyncSchedulerTimer({
  executeAutomaticSync,
  persistSyncSchedulerState,
  reportSyncSchedulerPersistenceFailure,
  stateRef,
  syncInFlight,
  syncScheduler,
  syncSchedulerHydrated,
  syncSchedulerRef
}: SyncSchedulerTimerOptions): void {
  useEffect(() => {
    if (!syncSchedulerHydrated || !syncScheduler.enabled) {
      return
    }
    const delayMs = nextSyncSchedulerWakeDelayMs(syncScheduler, currentUnixSeconds())
    if (delayMs === undefined) {
      return
    }

    const timer = window.setTimeout(() => {
      const schedulerState = syncSchedulerRef.current
      const nowUnixSeconds = currentUnixSeconds()
      if (!schedulerState.enabled || !isSyncSchedulerDue(schedulerState, nowUnixSeconds)) {
        return
      }
      const appState = stateRef.current
      const blockedReason = isSyncNowEnabled(appState)
        ? automaticSyncBlockedReason(appState)
        : automaticSyncBlockedReason(appState) ?? getMenuModel(appState).detail
      if (blockedReason !== undefined) {
        void persistSyncSchedulerState(
          markSyncSchedulerBlocked(schedulerState, {
            nowUnixSeconds,
            reason: blockedReason
          })
        ).catch((error: unknown) => {
          reportSyncSchedulerPersistenceFailure(error)
        })
        return
      }
      if (syncInFlight.current) {
        void persistSyncSchedulerState(markSyncSchedulerInFlightOverlap(schedulerState, nowUnixSeconds)).catch(
          (error: unknown) => {
            reportSyncSchedulerPersistenceFailure(error)
          }
        )
        return
      }
      executeAutomaticSync()
    }, delayMs)

    return () => window.clearTimeout(timer)
  }, [
    executeAutomaticSync,
    persistSyncSchedulerState,
    reportSyncSchedulerPersistenceFailure,
    stateRef,
    syncInFlight,
    syncScheduler,
    syncSchedulerHydrated,
    syncSchedulerRef
  ])
}
