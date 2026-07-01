import { describe, expect, it } from "vitest"
import {
  SYNC_SCHEDULER_INTERVAL_OPTIONS,
  changeSyncSchedulerInterval,
  createDefaultSyncSchedulerState,
  disableSyncScheduler,
  enableSyncScheduler,
  formatSyncSchedulerCooldownReason,
  formatSyncSchedulerLastResult,
  formatSyncSchedulerNextRun,
  isSyncSchedulerDue,
  markSyncSchedulerBlocked,
  markSyncSchedulerInFlightOverlap,
  markSyncSchedulerRetryableFailure,
  markSyncSchedulerRunning,
  markSyncSchedulerSuccess,
  parseSyncSchedulerIntervalSeconds,
  parseSyncSchedulerState,
  recoverHydratedSyncSchedulerState,
  type SyncSchedulerState
} from "./syncScheduler"

const NOW = 1_783_000_000

describe("sync scheduler", () => {
  it("creates a disabled default state when automatic sync has no native state", () => {
    // Given
    const nowUnixSeconds = NOW

    // When
    const state = createDefaultSyncSchedulerState(nowUnixSeconds)

    // Then
    expect(state).toEqual({
      enabled: false,
      interval_seconds: 1_800,
      status: "disabled",
      retry_attempt: 0,
      updated_at: nowUnixSeconds
    })
    expect(SYNC_SCHEDULER_INTERVAL_OPTIONS).toEqual([900, 1_800, 3_600])
    expect(isSyncSchedulerDue(state, nowUnixSeconds + 3_600)).toBe(false)
    expect(formatSyncSchedulerNextRun(state, nowUnixSeconds)).toBe("Automatic sync is off.")
    expect(formatSyncSchedulerLastResult(state)).toBe("No automatic sync has run yet.")
  })

  it("validates interval options before changing scheduler state", () => {
    // Given
    const disabled = createDefaultSyncSchedulerState(NOW)

    // When
    const interval = parseSyncSchedulerIntervalSeconds(900)
    const enabled = enableSyncScheduler(disabled, NOW)
    const changed = changeSyncSchedulerInterval(enabled, {
      intervalSeconds: interval,
      nowUnixSeconds: NOW + 5
    })

    // Then
    expect(changed.interval_seconds).toBe(900)
    expect(changed.retry_attempt).toBe(0)
    expect(changed.status).toBe("scheduled")
    expect(changed.next_run_at).toBe(NOW + 905)
    expect(() => parseSyncSchedulerIntervalSeconds(1)).toThrow("Invalid sync scheduler interval")
  })

  it("calculates scheduled and cooldown due states from explicit timestamps", () => {
    // Given
    const scheduled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)
    const cooldown: SyncSchedulerState = {
      ...scheduled,
      status: "cooldown",
      next_run_at: undefined,
      next_eligible_at: NOW + 60
    }

    // When / Then
    expect(isSyncSchedulerDue(scheduled, NOW + 1_799)).toBe(false)
    expect(isSyncSchedulerDue(scheduled, NOW + 1_800)).toBe(true)
    expect(isSyncSchedulerDue(cooldown, NOW + 59)).toBe(false)
    expect(isSyncSchedulerDue(cooldown, NOW + 60)).toBe(true)
    expect(formatSyncSchedulerNextRun(scheduled, NOW)).toBe("Next automatic sync in 30 min.")
    expect(formatSyncSchedulerNextRun(cooldown, NOW)).toBe("Retry automatic sync in 1 min.")
  })

  it("records running state and schedules the next run after success", () => {
    // Given
    const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)
    const running = markSyncSchedulerRunning(enabled, NOW + 10)

    // When
    const succeeded = markSyncSchedulerSuccess(running, NOW + 20)

    // Then
    expect(running).toMatchObject({
      status: "running",
      last_started_at: NOW + 10,
      next_run_at: undefined,
      next_eligible_at: undefined
    })
    expect(succeeded).toMatchObject({
      enabled: true,
      status: "scheduled",
      last_finished_at: NOW + 20,
      last_result: "success",
      retry_attempt: 0,
      next_run_at: NOW + 1_820,
      next_eligible_at: undefined
    })
    expect(formatSyncSchedulerLastResult(succeeded)).toBe("Last automatic sync succeeded.")
  })

  it("applies deterministic retryable failure backoff for attempts one two and six", () => {
    // Given
    const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)

    // When
    const attemptOne = markSyncSchedulerRetryableFailure(enabled, {
      nowUnixSeconds: NOW,
      randomUnit: 0.5,
      reason: "Network unavailable."
    })
    const attemptTwo = markSyncSchedulerRetryableFailure(attemptOne, {
      nowUnixSeconds: NOW + 10,
      randomUnit: 0.5,
      reason: "Network unavailable."
    })
    const attemptSix = markSyncSchedulerRetryableFailure(
      { ...attemptTwo, retry_attempt: 5 },
      { nowUnixSeconds: NOW + 20, randomUnit: 0.5, reason: "Provider timed out." }
    )

    // Then
    expect(attemptOne).toMatchObject({
      status: "cooldown",
      last_result: "retryable_failure",
      retry_attempt: 1,
      next_eligible_at: NOW + 60
    })
    expect(attemptTwo).toMatchObject({
      retry_attempt: 2,
      next_eligible_at: NOW + 130
    })
    expect(attemptSix).toMatchObject({
      retry_attempt: 6,
      next_eligible_at: NOW + 1_820
    })
    expect(formatSyncSchedulerCooldownReason(attemptOne)).toBe("Network unavailable.")
  })

  it("caps retryable failure backoff at thirty minutes", () => {
    // Given
    const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)

    // When
    const failed = markSyncSchedulerRetryableFailure(
      { ...enabled, retry_attempt: 30 },
      { nowUnixSeconds: NOW, randomUnit: 0.5, reason: "Provider timed out." }
    )

    // Then
    expect(failed.retry_attempt).toBe(31)
    expect(failed.next_eligible_at).toBe(NOW + 1_800)
  })

  it("records setup blockers without retry timestamps or disabling automatic sync", () => {
    // Given
    const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)

    // When
    const blocked = markSyncSchedulerBlocked(enabled, {
      nowUnixSeconds: NOW + 30,
      reason: "Finish Codex CLI setup in Settings before scanning."
    })

    // Then
    expect(blocked).toMatchObject({
      enabled: true,
      status: "blocked",
      last_result: "blocked",
      retry_attempt: 0,
      last_reason: "Finish Codex CLI setup in Settings before scanning.",
      next_run_at: undefined,
      next_eligible_at: undefined
    })
    expect(formatSyncSchedulerLastResult(blocked)).toBe("Last automatic sync needs action.")
  })

  it("reschedules in-flight overlap without incrementing retry attempts", () => {
    // Given
    const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)

    // When
    const overlapped = markSyncSchedulerInFlightOverlap(enabled, NOW + 40)

    // Then
    expect(overlapped).toMatchObject({
      enabled: true,
      status: "scheduled",
      retry_attempt: 0,
      next_run_at: NOW + 100,
      next_eligible_at: undefined,
      last_reason: "Waiting for current Sync Now to finish."
    })
  })

  it("recovers hydrated running state to cooldown without immediately running again", () => {
    // Given
    const running: SyncSchedulerState = {
      ...enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW),
      status: "running",
      last_started_at: NOW - 100,
      retry_attempt: 0
    }

    // When
    const recovered = recoverHydratedSyncSchedulerState(running, NOW)

    // Then
    expect(recovered).toMatchObject({
      enabled: true,
      status: "cooldown",
      retry_attempt: 1,
      next_eligible_at: NOW + 500,
      last_reason: "Morrow restarted before automatic sync finished."
    })
  })

  it("disables automatic sync by resetting scheduled retry state", () => {
    // Given
    const failed = markSyncSchedulerRetryableFailure(
      enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW),
      { nowUnixSeconds: NOW, randomUnit: 0.5, reason: "Network unavailable." }
    )

    // When
    const disabled = disableSyncScheduler(failed, NOW + 45)

    // Then
    expect(disabled).toMatchObject({
      enabled: false,
      status: "disabled",
      last_result: "manual_disabled",
      retry_attempt: 0,
      next_run_at: undefined,
      next_eligible_at: undefined,
      updated_at: NOW + 45
    })
  })

  it("clears cooldown after manual success and schedules the next interval", () => {
    // Given
    const failed = markSyncSchedulerRetryableFailure(
      enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW),
      { nowUnixSeconds: NOW, randomUnit: 0.5, reason: "Network unavailable." }
    )

    // When
    const succeeded = markSyncSchedulerSuccess(failed, NOW + 30)

    // Then
    expect(succeeded).toMatchObject({
      status: "scheduled",
      retry_attempt: 0,
      next_run_at: NOW + 1_830,
      next_eligible_at: undefined,
      last_reason: undefined
    })
  })

  it("throws when parsing malformed native interval or status payloads", () => {
    // Given
    const valid = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)

    // When / Then
    expect(() => parseSyncSchedulerState({ ...valid, interval_seconds: 1 })).toThrow(
      "Invalid sync scheduler state"
    )
    expect(() => parseSyncSchedulerState({ ...valid, status: "bogus" })).toThrow(
      "Invalid sync scheduler state"
    )
  })
})
