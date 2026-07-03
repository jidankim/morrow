import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  discoveredChat,
  resetAppShellBridgeTestHarness
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import {
  APP_SHELL_STATE_KEY,
  createBrowserShellStorage,
  createDefaultAppShellState,
  loadAppShellState
} from "./domain/appShell"

const NOW = 1_783_000_000

describe("App settings persistence", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("persists onboarding and settings across reloads", async () => {
    seedReadyStateWithReferenceTimezone("Asia/Seoul")
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

  it("renders system timezone options and persists an added concrete timezone", async () => {
    const supportedValuesOfSpy = vi.spyOn(Intl, "supportedValuesOf").mockImplementation((key) => {
      if (key === "timeZone") {
        return ["America/Los_Angeles", "Europe/Paris"]
      }
      return []
    })
    vi.stubEnv("TZ", "America/Los_Angeles")
    try {
      // Given: Settings opens with the system-default timezone preference.
      seedReadyStateWithReferenceTimezone("system")
      const firstRender = render(<App />)

      act(() => {
        window.location.hash = "#settings"
        window.dispatchEvent(new HashChangeEvent("hashchange"))
      })

      const referenceTimezoneSelect = screen.getByLabelText("Reference timezone")
      if (!(referenceTimezoneSelect instanceof HTMLSelectElement)) {
        throw new Error("Reference timezone control must be a select element")
      }
      const firstOption = referenceTimezoneSelect.options.item(0)
      if (firstOption === null) {
        throw new Error("Reference timezone select must render a first option")
      }

      // Then: the first option exposes the resolved browser timezone.
      expect(firstOption.value).toBe("system")
      expect(firstOption.textContent).toBe("System default (America/Los_Angeles)")
      expect(referenceTimezoneSelect).toHaveValue("system")

      // When: an expanded IANA timezone is selected.
      fireEvent.change(referenceTimezoneSelect, { target: { value: "America/Los_Angeles" } })

      // Then: the concrete timezone persists and survives a reload.
      await waitFor(() => {
        const reloaded = loadAppShellState(createBrowserShellStorage(window.localStorage))
        expect(reloaded.config.referenceTimezone).toBe("America/Los_Angeles")
      })

      firstRender.unmount()
      window.location.hash = "#status"
      render(<App />)

      await waitFor(() => expect(screen.getByTestId("sync-state")).toHaveTextContent("Enabled"))
      act(() => {
        window.location.hash = "#settings"
        window.dispatchEvent(new HashChangeEvent("hashchange"))
      })
      expect(screen.getByLabelText("Reference timezone")).toHaveValue("America/Los_Angeles")
    } finally {
      supportedValuesOfSpy.mockRestore()
      vi.unstubAllEnvs()
    }
  })

  it("renders settings with the safe fallback state when persisted timezone is malformed", async () => {
    window.localStorage.setItem(
      APP_SHELL_STATE_KEY,
      JSON.stringify({
        mode: "scanning",
        config: {
          referenceTimezone: "Mars/Olympus",
          calendarSource: "apple-calendar",
          permissionsGranted: true,
          launchAtLogin: false,
          sourceExcerptsEnabled: true,
          firstProposalGuidanceEnabled: true
        },
        selectedChats: [],
        pendingProposalCount: 0
      })
    )
    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(() => render(<App />)).not.toThrow()
    await waitFor(() => expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument())
  })

  it("persists opt-in local diagnostics settings without remote telemetry controls", async () => {
    seedReadyStateWithReferenceTimezone("Asia/Seoul")
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
    seedReadyStateWithReferenceTimezone("Asia/Seoul")
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
    const intervalInput = screen.getByLabelText("Automatic Sync interval")
    expect(intervalInput).toHaveValue(30)

    fireEvent.change(intervalInput, { target: { value: "7" } })
    await waitFor(() => expect(bridgeMock.getSchedulerState()).toMatchObject({ interval_seconds: 420 }))
    expect(intervalInput).toHaveValue(7)

    fireEvent.change(intervalInput, { target: { value: "1" } })
    await waitFor(() => expect(bridgeMock.getSchedulerState()).toMatchObject({ interval_seconds: 60 }))

    fireEvent.change(intervalInput, { target: { value: "0" } })
    expect(bridgeMock.getSchedulerState()).toMatchObject({ interval_seconds: 60 })
    fireEvent.blur(intervalInput)
    expect(intervalInput).toHaveValue(1)
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
      seedReadyStateWithReferenceTimezone("Asia/Seoul")

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
    seedReadyStateWithReferenceTimezone("Asia/Seoul")

    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    expect(screen.getByTestId("status-automatic-sync-status")).toHaveTextContent("Needs action")
    expect(screen.getByTestId("status-automatic-sync-reason")).toHaveTextContent(
      "Finish Codex CLI setup in Settings before scanning."
    )
  })
})

function seedReadyStateWithReferenceTimezone(referenceTimezone: string): void {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, referenceTimezone, permissionsGranted: false },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}
