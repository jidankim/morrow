import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createBrowserShellStorage, createDefaultAppShellState, loadAppShellState } from "./domain/appShell"

type NativeSyncSchedulerStateForTest = {
  readonly enabled: boolean
  readonly interval_seconds: 900 | 1_800 | 3_600
  readonly status: "disabled" | "scheduled" | "running" | "cooldown" | "blocked"
  readonly last_started_at?: number | undefined
  readonly last_finished_at?: number | undefined
  readonly next_run_at?: number | undefined
  readonly next_eligible_at?: number | undefined
  readonly last_result?: "success" | "retryable_failure" | "blocked" | "manual_disabled" | undefined
  readonly retry_attempt: number
  readonly last_reason?: string | undefined
  readonly updated_at: number
}

const NOW = 1_783_000_000

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_783_000_000
} as const

const bridgeMock = vi.hoisted(() => {
  let schedulerState: NativeSyncSchedulerStateForTest | undefined = {
    enabled: false,
    interval_seconds: 1_800,
    retry_attempt: 0,
    status: "disabled",
    updated_at: 1_783_000_000
  }
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    getRuntimeIdentity: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async () => vi.fn()),
    subscribeMenuCommand: vi.fn(async () => vi.fn()),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
    getSyncSchedulerState: vi.fn(async () => schedulerState),
    setSyncSchedulerState: vi.fn(async (state: NativeSyncSchedulerStateForTest) => {
      schedulerState = state
      return state
    }),
    resetSchedulerState: (): void => {
      schedulerState = {
        enabled: false,
        interval_seconds: 1_800,
        retry_attempt: 0,
        status: "disabled",
        updated_at: 1_783_000_000
      }
    },
    getSchedulerState: (): NativeSyncSchedulerStateForTest | undefined => schedulerState,
    setSchedulerState: (state: NativeSyncSchedulerStateForTest): void => {
      schedulerState = state
    },
    checkProviderAuth: vi.fn(async () => ({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })),
    storeMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: true, deleted: false })),
    readMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", present: true })),
    deleteMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: false, deleted: true })),
    discoverMessagesChats: vi.fn(async () => ({
      status: "ready",
      chats: [
        {
          chatId: "messages-chat-11111111111111111111111111111111",
          displayLabel: "Chat alpha",
          participantCount: 2,
          participantIds: [
            "messages-participant-11111111111111111111111111111111",
            "messages-participant-22222222222222222222222222222222"
          ],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    })),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
    deleteMorrowData: vi.fn(async () => undefined),
    recordCrashLog: vi.fn(async () => ({ stored: true }))
  }
})

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: false },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}

describe("App settings persistence", () => {
  beforeEach(() => {
    vi.useRealTimers()
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
    bridgeMock.resetSchedulerState()
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

  it("persists opt-in local diagnostics settings without remote telemetry controls", async () => {
    seedReadyState()
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(screen.getByText(/private local files on this Mac/i)).toBeInTheDocument()
    expect(screen.getByText(/not uploads/i)).toBeInTheDocument()
    expect(screen.getByText(/Delete All deletes these files/i)).toBeInTheDocument()
    expect(screen.queryByLabelText(/telemetry/i)).not.toBeInTheDocument()

    const diagnosticsToggle = screen.getByRole("checkbox", {
      name: "Write private local diagnostics files"
    })
    const retentionInput = screen.getByLabelText("Local diagnostics retention (days)")
    expect(diagnosticsToggle).not.toBeChecked()
    expect(retentionInput).toHaveValue(30)

    fireEvent.click(diagnosticsToggle)
    fireEvent.change(retentionInput, { target: { value: "14" } })

    await waitFor(() => {
      const reloaded = loadAppShellState(createBrowserShellStorage(window.localStorage))
      expect(reloaded.config.localDiagnosticsEnabled).toBe(true)
      expect(reloaded.config.localDiagnosticsRetentionDays).toBe(14)
      expect(reloaded.config.telemetryEnabled).toBe(false)
    })

    fireEvent.change(retentionInput, { target: { value: "0" } })
    fireEvent.change(retentionInput, { target: { value: "366" } })
    fireEvent.change(retentionInput, { target: { value: "abc" } })

    const reloaded = loadAppShellState(createBrowserShellStorage(window.localStorage))
    expect(reloaded.config.localDiagnosticsRetentionDays).toBe(14)
    expect(reloaded.config.telemetryEnabled).toBe(false)
  })

  it("persists automatic sync changes from Status and Settings controls", async () => {
    seedReadyState()
    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    expect(screen.getByTestId("status-automatic-sync-status")).toHaveTextContent("Off")

    fireEvent.click(screen.getByRole("button", { name: "Turn automatic sync on" }))

    await waitFor(() =>
      expect(bridgeMock.getSchedulerState()).toMatchObject({
        enabled: true,
        interval_seconds: 1_800,
        next_run_at: expect.any(Number),
        status: "scheduled"
      })
    )
    expect(screen.getByTestId("status-automatic-sync-status")).toHaveTextContent("Enabled")
    expect(screen.getByTestId("status-automatic-sync-next-run")).toHaveTextContent("Next automatic sync in 30 min.")

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.change(screen.getByLabelText("Automatic Sync interval"), { target: { value: "900" } })
    await waitFor(() => expect(bridgeMock.getSchedulerState()).toMatchObject({ interval_seconds: 900 }))

    fireEvent.change(screen.getByLabelText("Automatic Sync interval"), { target: { value: "3600" } })
    await waitFor(() => expect(bridgeMock.getSchedulerState()).toMatchObject({ interval_seconds: 3_600 }))
  })

  it("shows automatic cooldown without disabling manual Sync Now", async () => {
    const nowUnixSeconds = Math.floor(Date.now() / 1_000)
    const dateNowSpy = vi.spyOn(Date, "now").mockReturnValue(nowUnixSeconds * 1_000)
    try {
      bridgeMock.setSchedulerState({
        enabled: true,
        interval_seconds: 1_800,
        last_reason: "Provider timed out.",
        last_result: "retryable_failure",
        next_eligible_at: nowUnixSeconds + 60,
        retry_attempt: 1,
        status: "cooldown",
        updated_at: nowUnixSeconds
      })
      seedReadyState()

      render(<App />)

      await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
      expect(screen.getByTestId("status-automatic-sync-status")).toHaveTextContent("Cooling down")
      expect(screen.getByTestId("status-automatic-sync-next-run")).toHaveTextContent("Retry automatic sync in 1 min.")
      expect(screen.getByTestId("status-automatic-sync-reason")).toHaveTextContent("Provider timed out.")
    } finally {
      dateNowSpy.mockRestore()
    }
  })

  it("shows blocked automatic sync without disabling manual Sync Now when setup is ready", async () => {
    bridgeMock.setSchedulerState({
      enabled: true,
      interval_seconds: 1_800,
      last_reason: "Finish Codex CLI setup in Settings before scanning.",
      last_result: "blocked",
      retry_attempt: 0,
      status: "blocked",
      updated_at: NOW
    })
    seedReadyState()

    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    expect(screen.getByTestId("status-automatic-sync-status")).toHaveTextContent("Needs action")
    expect(screen.getByTestId("status-automatic-sync-reason")).toHaveTextContent(
      "Finish Codex CLI setup in Settings before scanning."
    )
  })
})
