import { act, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import type { SyncSchedulerState } from "./domain/syncScheduler"

type NativeMenuCommandForTest = "sync-now" | "open-settings" | "open-calendar" | "open-reminders"

const bridgeMock = vi.hoisted(() => {
  let nativeMenuCommandListener: ((command: NativeMenuCommandForTest) => void) | undefined
  const defaultSchedulerState: SyncSchedulerState = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  }
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    getRuntimeIdentity: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async () => vi.fn()),
    subscribeMenuCommand: vi.fn(async (listener: (command: NativeMenuCommandForTest) => void) => {
      nativeMenuCommandListener = listener
      return vi.fn()
    }),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
    getSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
    setSyncSchedulerState: vi.fn(async (state: SyncSchedulerState) => state),
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
          participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    })),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
    deleteMorrowData: vi.fn(async () => undefined),
    recordCrashLog: vi.fn(async () => ({ stored: true })),
    emitMenuCommand: (command: NativeMenuCommandForTest): void => {
      nativeMenuCommandListener?.(command)
    }
  }
})

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

describe("App native menu navigation", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.subscribeMenuCommand.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
  })

  it("opens Settings for native settings, calendar, and reminders commands", async () => {
    render(<App />)

    await waitFor(() => expect(bridgeMock.subscribeMenuCommand).toHaveBeenCalledOnce())
    expect(screen.getByRole("heading", { name: "App shell" })).toBeInTheDocument()

    act(() => bridgeMock.emitMenuCommand("open-settings"))
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
    expect(window.location.hash).toBe("#settings")

    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByRole("heading", { name: "App shell" })).toBeInTheDocument()

    act(() => bridgeMock.emitMenuCommand("open-calendar"))
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
})
