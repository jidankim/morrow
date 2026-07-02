import { useCallback, useEffect, useRef, useState } from "react"
import { isSyncNowEnabled, reduceAppShellState } from "./domain/appShell"
import {
  changeSyncSchedulerInterval,
  disableSyncScheduler,
  enableSyncScheduler,
  markSyncSchedulerRetryableFailure,
  markSyncSchedulerRunning,
  markSyncSchedulerSuccess,
  type SyncSchedulerIntervalSeconds,
  type SyncSchedulerState
} from "./domain/syncScheduler"
import { syncResultCountsFrom } from "./domain/syncResultCounts"
import { emptyDecisionEvidenceReport } from "./domain/decisionEvidence"
import { syncScanRequestFromState } from "./messagesDiscoveryBridge"
import { nativeErrorMessage } from "./nativeErrors"
import {
  currentUnixSeconds,
  hydrateSyncSchedulerState,
  syncSchedulerShellStateFields
} from "./syncSchedulerController"
import type {
  SyncExecutionReceipt,
  SyncSchedulerOrchestrator,
  SyncSchedulerOrchestratorOptions,
  SyncTrigger
} from "./useSyncSchedulerOrchestrator.types"
import { useSyncSchedulerTimer } from "./useSyncSchedulerTimer"

const DECISION_EVIDENCE_RECENT_LIMIT = 20

export function useSyncSchedulerOrchestrator({
  nativeBridge,
  state,
  setState,
  setSyncing,
  syncInFlight
}: SyncSchedulerOrchestratorOptions): SyncSchedulerOrchestrator {
  const [syncScheduler, setSyncScheduler] = useState<SyncSchedulerState>(
    () => hydrateSyncSchedulerState(undefined, currentUnixSeconds()).state
  )
  const [syncSchedulerHydrated, setSyncSchedulerHydrated] = useState(false)
  const stateRef = useRef(state)
  const syncSchedulerRef = useRef(syncScheduler)
  const schedulerShellState = syncSchedulerShellStateFields(syncScheduler, currentUnixSeconds())

  useEffect(() => {
    stateRef.current = state
  }, [state])

  useEffect(() => {
    syncSchedulerRef.current = syncScheduler
  }, [syncScheduler])

  const reportSyncSchedulerPersistenceFailure = useCallback(
    (error: unknown): string => {
      const message = nativeErrorMessage(error, "Sync scheduler state could not be saved.")
      setState((current) => reduceAppShellState(current, { type: "fail", message }))
      return message
    },
    [setState]
  )

  const persistSyncSchedulerState = useCallback(
    async (
      nextState: SyncSchedulerState,
      rollbackState = syncSchedulerRef.current
    ): Promise<SyncSchedulerState> => {
      setSyncScheduler(nextState)
      syncSchedulerRef.current = nextState
      try {
        const persistedState = await nativeBridge.setSyncSchedulerState(nextState)
        if (persistedState === undefined) {
          return nextState
        }
        setSyncScheduler(persistedState)
        syncSchedulerRef.current = persistedState
        return persistedState
      } catch (error: unknown) {
        setSyncScheduler(rollbackState)
        syncSchedulerRef.current = rollbackState
        throw error
      }
    },
    [nativeBridge]
  )

  useEffect(() => {
    let active = true
    void nativeBridge
      .getSyncSchedulerState()
      .then((persistedState) => {
        if (!active) {
          return
        }
        const nowUnixSeconds = currentUnixSeconds()
        const hydrated = hydrateSyncSchedulerState(persistedState, nowUnixSeconds)
        if (hydrated.shouldPersist && persistedState !== undefined) {
          void persistSyncSchedulerState(hydrated.state, persistedState).catch((error: unknown) => {
            if (!active) {
              return
            }
            reportSyncSchedulerPersistenceFailure(error)
          })
        } else {
          setSyncScheduler(hydrated.state)
          syncSchedulerRef.current = hydrated.state
        }
        setSyncSchedulerHydrated(true)
      })
      .catch((error: unknown) => {
        const message = nativeErrorMessage(error, "Sync scheduler state could not be read.")
        setState((current) => reduceAppShellState(current, { type: "fail", message }))
      })
    return () => {
      active = false
    }
  }, [nativeBridge, persistSyncSchedulerState, reportSyncSchedulerPersistenceFailure, setState])

  const executeSyncNow = useCallback(
    async (trigger: SyncTrigger): Promise<SyncExecutionReceipt> => {
      const syncState = stateRef.current
      if (!isSyncNowEnabled(syncState)) {
        return { status: "skipped-disabled" }
      }
      if (syncInFlight.current) {
        return { status: "skipped-in-flight" }
      }

      syncInFlight.current = true
      setSyncing(true)
      setState((current) => reduceAppShellState(current, { type: "clearDecisionEvidence" }))

      try {
        if (trigger === "automatic") {
          try {
            await persistSyncSchedulerState(markSyncSchedulerRunning(syncSchedulerRef.current, currentUnixSeconds()))
          } catch (error: unknown) {
            if (!(error instanceof Error) && typeof error !== "string") {
              const message = reportSyncSchedulerPersistenceFailure(error)
              return { status: "failed", message }
            }
            const message = reportSyncSchedulerPersistenceFailure(error)
            return { status: "failed", message }
          }
        }

        await nativeBridge.reconcileNow()
        const result = await nativeBridge.scanSelectedChats(syncScanRequestFromState(syncState))
        const counts = syncResultCountsFrom(result)
        const decisionEvidence =
          result === undefined
            ? emptyDecisionEvidenceReport
            : (await nativeBridge.loadDecisionEvidence({
                createdCandidateIds: result.createdCandidateIds,
                limit: DECISION_EVIDENCE_RECENT_LIMIT
              })) ?? emptyDecisionEvidenceReport
        setState((current) =>
          reduceAppShellState(current, {
            type: "syncCompleted",
            decisionEvidence,
            ...counts
          })
        )
        if (syncSchedulerRef.current.enabled) {
          try {
            await persistSyncSchedulerState(markSyncSchedulerSuccess(syncSchedulerRef.current, currentUnixSeconds()))
          } catch (error: unknown) {
            if (!(error instanceof Error) && typeof error !== "string") {
              const message = reportSyncSchedulerPersistenceFailure(error)
              return { status: "failed", message }
            }
            const message = reportSyncSchedulerPersistenceFailure(error)
            return { status: "failed", message }
          }
        }
        return { status: "completed" }
      } catch (error: unknown) {
        const message =
          error instanceof Error || typeof error === "string"
            ? nativeErrorMessage(error, "Sync Now could not complete.")
            : "Sync Now could not complete."
        if (trigger === "automatic") {
          await persistSyncSchedulerState(
            markSyncSchedulerRetryableFailure(syncSchedulerRef.current, {
              nowUnixSeconds: currentUnixSeconds(),
              randomUnit: Math.random(),
              reason: message
            })
          ).catch((persistenceError: unknown) => {
            reportSyncSchedulerPersistenceFailure(persistenceError)
          })
        } else {
          setState((current) => reduceAppShellState(current, { type: "fail", message }))
        }
        return { status: "failed", message }
      } finally {
        syncInFlight.current = false
        setSyncing(false)
      }
    },
    [nativeBridge, persistSyncSchedulerState, reportSyncSchedulerPersistenceFailure, setState, setSyncing, syncInFlight]
  )

  const executeAutomaticSync = useCallback((): void => {
    void executeSyncNow("automatic")
  }, [executeSyncNow])

  useSyncSchedulerTimer({
    executeAutomaticSync,
    persistSyncSchedulerState,
    reportSyncSchedulerPersistenceFailure,
    stateRef,
    syncInFlight,
    syncScheduler,
    syncSchedulerHydrated,
    syncSchedulerRef
  })

  const runSyncNow = useCallback((): void => {
    void executeSyncNow("manual")
  }, [executeSyncNow])

  const persistUserSchedulerChange = useCallback(
    (nextState: SyncSchedulerState): void => {
      void persistSyncSchedulerState(nextState).catch((error: unknown) => {
        reportSyncSchedulerPersistenceFailure(error)
      })
    },
    [persistSyncSchedulerState, reportSyncSchedulerPersistenceFailure]
  )

  const toggleAutomaticSync = useCallback((): void => {
    const nowUnixSeconds = currentUnixSeconds()
    const current = syncSchedulerRef.current
    persistUserSchedulerChange(
      current.enabled
        ? disableSyncScheduler(current, nowUnixSeconds)
        : enableSyncScheduler(current, nowUnixSeconds)
    )
  }, [persistUserSchedulerChange])

  const changeAutomaticSyncInterval = useCallback(
    (intervalSeconds: SyncSchedulerIntervalSeconds): void => {
      persistUserSchedulerChange(
        changeSyncSchedulerInterval(syncSchedulerRef.current, {
          intervalSeconds,
          nowUnixSeconds: currentUnixSeconds()
        })
      )
    },
    [persistUserSchedulerChange]
  )

  return {
    changeAutomaticSyncInterval,
    runSyncNow,
    schedulerShellState,
    syncScheduler,
    syncSchedulerNowUnixSeconds: currentUnixSeconds(),
    toggleAutomaticSync
  }
}
