import { act, fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"

const NOW = 1_783_000_000

async function advanceSchedulerBy(milliseconds: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(milliseconds)
  })
}

async function flushAsyncEffects(): Promise<void> {
  await act(async () => {
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
  })
}

describe("App scheduled sync persistence failures", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("surfaces automatic pre-scan scheduler persistence rejection and releases manual Sync Now", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    bridgeMock.setSyncSchedulerState.mockRejectedValueOnce(new Error("Native scheduler write failed."))
    bridgeMock.setSchedulerState({
      enabled: true,
      interval_seconds: 1_800,
      status: "scheduled",
      next_run_at: NOW + 1_800,
      retry_attempt: 0,
      updated_at: NOW
    })
    seedReadyState()
    render(<App />)
    await flushAsyncEffects()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled()

    // When
    await advanceSchedulerBy(1_800_000)
    await flushAsyncEffects()

    // Then
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
    expect(screen.getByTestId("status-label")).toHaveTextContent("Error")
    expect(screen.getByText("Native scheduler write failed.")).toBeInTheDocument()

    bridgeMock.resetSyncCalls()
    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))

    await flushAsyncEffects()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
  })

  it("surfaces blocked scheduler persistence rejection without scanning or keeping unpersisted blocked state", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "notLoggedIn",
      ready: false,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI is not logged in."
    })
    bridgeMock.setSyncSchedulerState.mockRejectedValueOnce(new Error("Native scheduler blocked save failed."))
    bridgeMock.setSchedulerState({
      enabled: true,
      interval_seconds: 1_800,
      status: "scheduled",
      next_run_at: NOW + 1_800,
      retry_attempt: 0,
      updated_at: NOW
    })
    seedReadyState()
    render(<App />)
    await flushAsyncEffects()
    expect(screen.getAllByText("Finish Codex CLI setup in Settings before scanning.").length).toBeGreaterThan(0)

    // When
    await advanceSchedulerBy(1_800_000)
    await flushAsyncEffects()

    // Then
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
    expect(screen.getByTestId("status-label")).toHaveTextContent("Error")
    expect(screen.getByText("Native scheduler blocked save failed.")).toBeInTheDocument()
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "scheduled",
      next_run_at: NOW + 1_800
    })
    expect(bridgeMock.setShellState).toHaveBeenLastCalledWith(
      expect.objectContaining({
        mode: "error",
        errorMessage: "Native scheduler blocked save failed.",
        automaticSyncStatusLabel: "On",
        automaticSyncDetail: "Next automatic sync now."
      })
    )
  })
})
