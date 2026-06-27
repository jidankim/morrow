import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

type NativeStateForTest = {
  readonly mode: "scanning" | "paused" | "error"
  readonly errorMessage?: string
  readonly onboardingComplete: boolean
  readonly pendingProposalCount: number
}

type NativeMenuCommandForTest = "sync-now" | "open-settings" | "open-calendar" | "open-reminders"

const bridgeMock = vi.hoisted(() => {
  let nativeStateListener: ((state: NativeStateForTest) => void) | undefined
  let nativeMenuCommandListener: ((command: NativeMenuCommandForTest) => void) | undefined
  const syncCalls: string[] = []
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async (listener: (state: NativeStateForTest) => void) => {
      nativeStateListener = listener
      return vi.fn()
    }),
    subscribeMenuCommand: vi.fn(async (listener: (command: NativeMenuCommandForTest) => void) => {
      nativeMenuCommandListener = listener
      return vi.fn()
    }),
    reconcileNow: vi.fn(async () => {
      syncCalls.push("reconcile")
    }),
    scanSelectedChats: vi.fn(async () => {
      syncCalls.push("scan")
      return { pendingProposalCount: 12 }
    }),
    emitNativeState: (state: NativeStateForTest): void => {
      nativeStateListener?.(state)
    },
    emitMenuCommand: (command: NativeMenuCommandForTest): void => {
      nativeMenuCommandListener?.(command)
    },
    getSyncCalls: (): readonly string[] => syncCalls,
    resetSyncCalls: (): void => {
      syncCalls.length = 0
    }
  }
})

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  createNativeShellBridge: () => ({
    getState: bridgeMock.getState,
    setShellState: bridgeMock.setShellState,
    subscribeAppState: bridgeMock.subscribeAppState,
    subscribeMenuCommand: bridgeMock.subscribeMenuCommand,
    reconcileNow: bridgeMock.reconcileNow,
    scanSelectedChats: bridgeMock.scanSelectedChats
  })
}))

const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: true },
      selectedChats: [{ id: "chat-alpha", label: "Chat alpha", backfillPromptEnabled: true }]
    })
  )
}

describe("App native shell bridge", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.getState.mockClear()
    bridgeMock.setShellState.mockClear()
    bridgeMock.subscribeAppState.mockClear()
    bridgeMock.subscribeMenuCommand.mockClear()
    bridgeMock.reconcileNow.mockClear()
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.resetSyncCalls()
  })

  it("persists native menu pause events and disables Sync Now", async () => {
    render(<App />)

    await waitFor(() => expect(bridgeMock.subscribeAppState).toHaveBeenCalledOnce())
    act(() => {
      bridgeMock.emitNativeState({
        mode: "paused",
        onboardingComplete: false,
        pendingProposalCount: 0
      })
    })

    await expect(screen.findByTestId("status-label")).resolves.toHaveTextContent("Paused")
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()

    const stored = window.localStorage.getItem(APP_SHELL_STATE_KEY)
    expect(stored).not.toBeNull()
    expect(JSON.parse(stored ?? "{}")).toMatchObject({ mode: "paused" })
  })

  it("blocks Sync Now until required setup and at least one chat are selected", async () => {
    render(<App />)

    await screen.findByText("Onboarding required")
    expect(screen.getByText("Native chat discovery unavailable.")).toBeInTheDocument()
    expect(screen.getByText("Select at least one chat before scanning.")).toBeInTheDocument()
    expect(screen.getByTestId("status-label")).toHaveTextContent("Setup needed")
    await waitFor(() =>
      expect(bridgeMock.setShellState).toHaveBeenCalledWith({
        mode: "scanning",
        errorMessage: undefined,
        onboardingComplete: false,
        pendingProposalCount: 0
      })
    )

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    expect(syncButton).toBeDisabled()
    fireEvent.click(syncButton)

    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
  })

  it("renders production menu commands without debug-only error controls", async () => {
    render(<App />)

    await screen.findByText("Onboarding required")

    expect(screen.getByRole("link", { name: "Settings" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Pause" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    expect(screen.getByTestId("pending-count")).toHaveTextContent("0")
    expect(screen.queryByRole("button", { name: "Simulate Error" })).not.toBeInTheDocument()
  })

  it("runs Sync Now as reconcile then scan once across rapid clicks", async () => {
    seedReadyState()
    render(<App />)

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())

    fireEvent.click(syncButton)
    fireEvent.click(syncButton)

    await waitFor(() => expect(screen.getByTestId("pending-count")).toHaveTextContent("9+"))
    expect(bridgeMock.getSyncCalls()).toEqual(["reconcile", "scan"])
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce()
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: ["chat-alpha"],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["chat-alpha"],
      sourceExcerptsEnabled: true,
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

  it("opens Settings for native settings, calendar, and reminders commands", async () => {
    render(<App />)

    await waitFor(() => expect(bridgeMock.subscribeMenuCommand).toHaveBeenCalledOnce())
    expect(screen.getByRole("heading", { name: "App shell" })).toBeInTheDocument()

    act(() => {
      bridgeMock.emitMenuCommand("open-settings")
    })
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
    expect(window.location.hash).toBe("#settings")

    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByRole("heading", { name: "App shell" })).toBeInTheDocument()

    act(() => {
      bridgeMock.emitMenuCommand("open-calendar")
    })
    expect(screen.getByLabelText("Calendar source")).toBeInTheDocument()
    expect(screen.queryByRole("option", { name: "ICS feed" })).not.toBeInTheDocument()
    expect(window.location.hash).toBe("#settings")

    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
      bridgeMock.emitMenuCommand("open-reminders")
    })
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
    expect(window.location.hash).toBe("#settings")
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
      expect(stored).toContain("chat-alpha")
    })

    firstRender.unmount()
    window.location.hash = "#status"
    render(<App />)

    expect(screen.getByTestId("sync-state")).toHaveTextContent("Enabled")
    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByLabelText("Reference timezone")).toHaveValue("America/New_York")
    expect(screen.getByLabelText("Open Morrow at login")).toBeChecked()
  })

})
