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

type NativeDiscoveryReportForTest =
  | {
      readonly status: "ready"
      readonly chats: readonly {
        readonly chatId: string
        readonly displayLabel: string
        readonly participantCount: number
        readonly participantIds: readonly string[]
        readonly latestActivityTimestamp: number
      }[]
    }
  | { readonly status: "empty" | "permissionDenied" | "unavailable"; readonly chats: readonly [] }

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
  latestActivityTimestamp: 1_783_000_000
} as const

const nativeReadyReport = {
  status: "ready",
  chats: [
    {
      chatId: "messages-chat-11111111111111111111111111111111",
      displayLabel: "Chat alpha",
      participantCount: 2,
      participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
      latestActivityTimestamp: 1_783_000_000
    }
  ]
} as const satisfies NativeDiscoveryReportForTest

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
    storeMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: true,
      deleted: false
    })),
    readMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      present: true
    })),
    deleteMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: false,
      deleted: true
    })),
    discoverMessagesChats: vi.fn(async (): Promise<NativeDiscoveryReportForTest> => nativeReadyReport),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
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
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => ({
    getState: bridgeMock.getState,
    setShellState: bridgeMock.setShellState,
    subscribeAppState: bridgeMock.subscribeAppState,
    subscribeMenuCommand: bridgeMock.subscribeMenuCommand,
    reconcileNow: bridgeMock.reconcileNow,
    scanSelectedChats: bridgeMock.scanSelectedChats,
    storeMorrowToken: bridgeMock.storeMorrowToken,
    readMorrowToken: bridgeMock.readMorrowToken,
    deleteMorrowToken: bridgeMock.deleteMorrowToken,
    discoverMessagesChats: bridgeMock.discoverMessagesChats,
    openPrivacySettings: bridgeMock.openPrivacySettings
  })
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
    bridgeMock.storeMorrowToken.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.readMorrowToken.mockResolvedValue({
      storageSurface: "keychainBridge",
      present: true
    })
    bridgeMock.deleteMorrowToken.mockClear()
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
    bridgeMock.openPrivacySettings.mockClear()
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
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [discoveredChat],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
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

})
