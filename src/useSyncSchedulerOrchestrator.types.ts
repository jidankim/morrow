import type { Dispatch, MutableRefObject, SetStateAction } from "react"
import type { AppShellState } from "./domain/appShell"
import type { SyncSchedulerIntervalSeconds, SyncSchedulerState } from "./domain/syncScheduler"
import type { SchedulerShellStateFields } from "./syncSchedulerController"
import type { NativeShellBridge } from "./tauriBridge"

export type SyncTrigger = "manual" | "automatic"

export type SyncExecutionReceipt =
  | { readonly status: "completed" }
  | { readonly status: "failed"; readonly message: string }
  | { readonly status: "skipped-disabled" }
  | { readonly status: "skipped-in-flight" }

export type SyncSchedulerOrchestratorOptions = {
  readonly nativeBridge: NativeShellBridge
  readonly state: AppShellState
  readonly setState: Dispatch<SetStateAction<AppShellState>>
  readonly setSyncing: Dispatch<SetStateAction<boolean>>
  readonly syncInFlight: MutableRefObject<boolean>
}

export type SyncSchedulerOrchestrator = {
  readonly changeAutomaticSyncInterval: (intervalSeconds: SyncSchedulerIntervalSeconds) => void
  readonly runSyncNow: () => void
  readonly syncScheduler: SyncSchedulerState
  readonly syncSchedulerNowUnixSeconds: number
  readonly schedulerShellState: SchedulerShellStateFields
  readonly toggleAutomaticSync: () => void
}
