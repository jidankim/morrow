import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import { syncSchedulerIntervalSecondsSchema } from "./domain/syncSchedulerInterval"

const syncSchedulerStatusSchema = z.union([
  z.literal("disabled"),
  z.literal("scheduled"),
  z.literal("running"),
  z.literal("cooldown"),
  z.literal("blocked")
])

const syncSchedulerLastResultSchema = z.union([
  z.literal("success"),
  z.literal("retryable_failure"),
  z.literal("blocked"),
  z.literal("manual_disabled")
])

const syncSchedulerStateSchema = z.object({
  enabled: z.boolean(),
  interval_seconds: syncSchedulerIntervalSecondsSchema,
  status: syncSchedulerStatusSchema,
  last_started_at: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.number().int().nonnegative().optional()
  ),
  last_finished_at: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.number().int().nonnegative().optional()
  ),
  next_run_at: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.number().int().nonnegative().optional()
  ),
  next_eligible_at: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.number().int().nonnegative().optional()
  ),
  last_result: z.preprocess(
    (value) => (value === null ? undefined : value),
    syncSchedulerLastResultSchema.optional()
  ),
  retry_attempt: z.number().int().min(0),
  last_reason: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.string().min(1).optional()
  ),
  updated_at: z.number().int().nonnegative()
})

export type SyncSchedulerState = z.infer<typeof syncSchedulerStateSchema>

export function parseSyncSchedulerState(value: unknown): SyncSchedulerState {
  return syncSchedulerStateSchema.parse(value)
}

export async function getSyncSchedulerStateInTauri(): Promise<SyncSchedulerState> {
  return parseSyncSchedulerState(await invoke<unknown>("get_sync_scheduler_state"))
}

export async function setSyncSchedulerStateInTauri(
  state: SyncSchedulerState
): Promise<SyncSchedulerState> {
  return parseSyncSchedulerState(
    await invoke<unknown>("set_sync_scheduler_state", {
      state: parseSyncSchedulerState(state)
    })
  )
}
