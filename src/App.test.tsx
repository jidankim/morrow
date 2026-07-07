import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  discoveredChat,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import { APP_SHELL_STATE_KEY } from "./domain/appShell"

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

describe("App native shell bridge", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("persists native menu pause events and disables Sync Now", async () => {
    render(<App />)

    await waitFor(() => expect(bridgeMock.subscribeAppState).toHaveBeenCalledOnce())
    act(() => {
      bridgeMock.emitNativeState({
        mode: "paused",
        onboardingComplete: false,
        pendingProposalCount: 0,
        syncNowRunning: false,
        automaticSyncEnabled: false,
        automaticSyncStatusLabel: "Off",
        automaticSyncDetail: "Automatic sync is off."
      })
    })

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Sync Now disabled")
    expect(screen.getByRole("button", { name: "Enable Sync Now" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()

    const stored = window.localStorage.getItem(APP_SHELL_STATE_KEY)
    expect(stored).not.toBeNull()
    expect(JSON.parse(stored ?? "{}")).toMatchObject({ mode: "paused" })
  })

  it("renders production menu commands without debug-only error controls", async () => {
    render(<App />)

    await screen.findByText("Onboarding required")

    expect(screen.getByRole("link", { name: "Settings" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Disable Sync Now" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    expect(screen.getByTestId("pending-count")).toHaveTextContent("0")
    expect(screen.queryByRole("button", { name: "Simulate Error" })).not.toBeInTheDocument()
  })

  it("runs Sync Now as reconcile then scan once across rapid clicks", async () => {
    seedReadyState()
    render(<App />)

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())
    expect(screen.getByTestId("status-label")).toHaveTextContent("Ready")
    expect(screen.getByText("Ready. Use Sync Now to reconcile calendars and scan selected chats.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Disable Sync Now" })).toBeInTheDocument()

    fireEvent.click(syncButton)
    fireEvent.click(syncButton)

    await waitFor(() => expect(screen.getByTestId("pending-count")).toHaveTextContent("9+"))
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [discoveredChat],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: expect.any(Number),
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: false,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listIntakeProfiles: [],
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 0
      }
    })
  })

  it("runs native Sync Now commands through reconcile then scan once", async () => {
    seedReadyState()
    render(<App />)

    await waitFor(() => expect(bridgeMock.subscribeMenuCommand).toHaveBeenCalledOnce())

    act(() => {
      bridgeMock.emitMenuCommand("sync-now")
      bridgeMock.emitMenuCommand("sync-now")
    })

    await waitFor(() => expect(screen.getByTestId("pending-count")).toHaveTextContent("9+"))
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
  })

  it("toggles persisted automatic sync from native menu without running sync", async () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date(NOW * 1_000))
    seedReadyState()
    render(<App />)

    await flushAsyncEffects()
    expect(bridgeMock.subscribeMenuCommand).toHaveBeenCalledOnce()
    expect(bridgeMock.getSyncSchedulerState).toHaveBeenCalledOnce()
    bridgeMock.setSyncSchedulerState.mockClear()
    bridgeMock.setShellState.mockClear()

    // When
    act(() => {
      bridgeMock.emitMenuCommand("toggle-automatic-sync")
    })

    // Then
    await flushAsyncEffects()
    expect(bridgeMock.setSyncSchedulerState).toHaveBeenCalledOnce()
    expect(bridgeMock.setSyncSchedulerState).toHaveBeenCalledWith({
      enabled: true,
      interval_seconds: 1_800,
      status: "scheduled",
      next_run_at: NOW + 1_800,
      retry_attempt: 0,
      updated_at: NOW
    })
    expect(bridgeMock.getSyncCalls()).toEqual([])
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
    expect(bridgeMock.setShellState).toHaveBeenLastCalledWith(
      expect.objectContaining({
        automaticSyncEnabled: true,
        automaticSyncStatusLabel: "On",
        automaticSyncDetail: "Next automatic sync in 30 min."
      })
    )
  })

  it("records needs action after native automatic sync toggle becomes due without provider credentials", async () => {
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
    seedReadyState()
    render(<App />)

    await flushAsyncEffects()
    expect(bridgeMock.subscribeMenuCommand).toHaveBeenCalledOnce()
    expect(screen.getAllByText("Finish Codex CLI setup in Settings before scanning.").length).toBeGreaterThan(0)

    // When
    act(() => {
      bridgeMock.emitMenuCommand("toggle-automatic-sync")
    })
    await flushAsyncEffects()
    expect(bridgeMock.getSchedulerState()).toMatchObject({
      enabled: true,
      status: "scheduled",
      next_run_at: NOW + 1_800
    })
    await advanceSchedulerBy(1_800_000)
    await flushAsyncEffects()

    // Then
    expect(bridgeMock.getSyncCalls()).toEqual([])
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

  it("shows an error state when Sync Now rejects with a native string error", async () => {
    bridgeMock.reconcileNow.mockRejectedValueOnce("native sync failed")
    seedReadyState()
    render(<App />)

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())
    fireEvent.click(syncButton)

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Error")
    expect(screen.getByText("native sync failed")).toBeInTheDocument()
  })

  it("shows fallback text when manual Sync Now rejects with an unknown error", async () => {
    // Given
    bridgeMock.reconcileNow.mockRejectedValueOnce({ reason: "not an Error" })
    seedReadyState()
    render(<App />)

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())

    // When
    fireEvent.click(syncButton)

    // Then
    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Error")
    expect(screen.getByText("Sync Now could not complete.")).toBeInTheDocument()
  })

  it("shows an error state when native shell state rejects with a string error", async () => {
    bridgeMock.getState.mockRejectedValueOnce("native state unavailable")
    render(<App />)

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Error")
    expect(screen.getByText("native state unavailable")).toBeInTheDocument()
  })

  it("shows fallback text when native shell state rejects with an unknown error", async () => {
    bridgeMock.getState.mockRejectedValueOnce({ reason: "not an Error" })
    render(<App />)

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Error")
    expect(screen.getByText("Native app shell state could not be read.")).toBeInTheDocument()
  })

  it("shows native menu subscription string failures", async () => {
    bridgeMock.subscribeMenuCommand.mockRejectedValueOnce("menu stream unavailable")
    render(<App />)

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Error")
    expect(screen.getByText("menu stream unavailable")).toBeInTheDocument()
  })

  it("persists onboarding and settings across reloads", async () => {
    seedReadyState()
    const firstRender = render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.change(screen.getByLabelText("Reference timezone"), {
      target: { value: "America/New_York" }
    })
    fireEvent.click(screen.getByLabelText("Open Morrow at login"))

    await waitFor(() => {
      const stored = window.localStorage.getItem(APP_SHELL_STATE_KEY)
      expect(stored).toContain("America/New_York")
      expect(stored).toContain("messages-chat-11111111111111111111111111111111")
    })

    firstRender.unmount()
    window.location.hash = "#status"
    render(<App />)

    await waitFor(() => expect(screen.getByTestId("sync-state")).toHaveTextContent("Enabled"))
    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByLabelText("Reference timezone")).toHaveValue("America/New_York")
    expect(screen.getByLabelText("Open Morrow at login")).toBeChecked()
  })

})
