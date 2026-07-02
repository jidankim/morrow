import { act, fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"

const NOW = 1_783_000_000

async function flushAsyncEffects(): Promise<void> {
  await act(async () => {
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
  })
}

describe("App scheduled sync recovery", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("recovers hydrated running scheduler state to cooldown before any scan", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    bridgeMock.setSchedulerState({
      enabled: true,
      interval_seconds: 1_800,
      status: "running",
      last_started_at: NOW - 100,
      retry_attempt: 0,
      updated_at: NOW - 100
    })
    seedReadyState()

    // When
    render(<App />)
    await flushAsyncEffects()
    expect(bridgeMock.setSyncSchedulerState).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "cooldown",
        retry_attempt: 1,
        next_eligible_at: NOW + 500,
        last_reason: "Morrow restarted before automatic sync finished."
      })
    )

    // Then
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
  })

  it("lets manual Sync Now run during automatic cooldown and clears retry state on success", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    bridgeMock.setSchedulerState({
      enabled: true,
      interval_seconds: 1_800,
      status: "cooldown",
      next_eligible_at: NOW + 600,
      last_result: "retryable_failure",
      retry_attempt: 2,
      last_reason: "Provider timed out.",
      updated_at: NOW
    })
    seedReadyState()
    render(<App />)
    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await flushAsyncEffects()
    expect(syncButton).toBeEnabled()

    // When
    fireEvent.click(syncButton)

    // Then
    await flushAsyncEffects()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "scheduled",
      last_result: "success",
      retry_attempt: 0,
      next_eligible_at: undefined,
      last_reason: undefined,
      next_run_at: NOW + 1_800
    })
  })
})
