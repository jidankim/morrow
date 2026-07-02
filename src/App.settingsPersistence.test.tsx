import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createBrowserShellStorage, loadAppShellState } from "./domain/appShell"

const NOW = 1_783_000_000

describe("App settings persistence", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
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
