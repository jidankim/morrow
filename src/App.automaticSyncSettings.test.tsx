import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  bridgeMock,
  discoveredChat,
  resetAppShellBridgeTestHarness
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

const NOW = 1_783_000_000

describe("App automatic sync settings", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
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
    expect(screen.getByTestId("status-automatic-sync-next-run")).toHaveTextContent(
      "Next automatic sync in 30 min."
    )

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
      expect(screen.getByTestId("status-automatic-sync-next-run")).toHaveTextContent(
        "Retry automatic sync in 1 min."
      )
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
