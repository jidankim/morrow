import { act, fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  discoveredChat,
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

describe("App scheduled sync", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("keeps automatic sync off by default without scheduling scans", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    bridgeMock.setSchedulerState(undefined)

    // When
    render(<App />)
    await advanceSchedulerBy(3_600_000)

    // Then
    expect(bridgeMock.getSyncSchedulerState).toHaveBeenCalledOnce()
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
    expect(bridgeMock.setShellState).toHaveBeenLastCalledWith(
      expect.objectContaining({
        automaticSyncEnabled: false,
        automaticSyncStatusLabel: "Off",
        automaticSyncDetail: "Automatic sync is off."
      })
    )
  })

  it("runs one automatic reconcile and scan when the persisted scheduler becomes due", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
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
    await advanceSchedulerBy(0)

    // Then
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [discoveredChat],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: NOW + 1_800,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: false,
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 0
      }
    })
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "scheduled",
      last_result: "success",
      retry_attempt: 0,
      next_run_at: NOW + 3_600
    })
  })

  it("reschedules automatic sync when a due timer overlaps manual Sync Now in flight", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    let finishScan = (): void => undefined
    bridgeMock.scanSelectedChats.mockImplementationOnce(
      async () =>
        await new Promise((resolve: (value: { readonly pendingProposalCount: number }) => void) => {
          finishScan = () => resolve({ pendingProposalCount: 12 })
        })
    )
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
    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await flushAsyncEffects()
    expect(syncButton).toBeEnabled()

    // When
    fireEvent.click(syncButton)
    await flushAsyncEffects()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    await advanceSchedulerBy(1_800_000)

    // Then
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "scheduled",
      retry_attempt: 0,
      next_run_at: NOW + 1_860,
      last_reason: "Waiting for current Sync Now to finish."
    })

    await act(async () => {
      finishScan()
    })
  })

  it("records blocked readiness without automatic scan calls", async () => {
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

    // Then
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "blocked",
      last_result: "blocked",
      last_reason: "Finish Codex CLI setup in Settings before scanning."
    })
    expect(bridgeMock.setShellState).toHaveBeenLastCalledWith(
      expect.objectContaining({
        automaticSyncStatusLabel: "Needs Action",
        automaticSyncDetail: "Finish Codex CLI setup in Settings before scanning."
      })
    )
  })

  it("records cooldown when an automatic scan rejects transiently", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    vi.spyOn(Math, "random").mockReturnValue(0.5)
    bridgeMock.scanSelectedChats.mockRejectedValueOnce(new Error("Provider timed out."))
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

    // Then
    expect(bridgeMock.reconcileNow).toHaveBeenCalledOnce()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      status: "cooldown",
      last_result: "retryable_failure",
      retry_attempt: 1,
      next_eligible_at: NOW + 1_860,
      last_reason: "Provider timed out."
    })
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
