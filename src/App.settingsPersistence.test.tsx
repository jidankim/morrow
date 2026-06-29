import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

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

const bridgeMock = vi.hoisted(() => ({
  getState: vi.fn(async () => undefined),
  setShellState: vi.fn(async () => undefined),
  subscribeAppState: vi.fn(async () => vi.fn()),
  subscribeMenuCommand: vi.fn(async () => vi.fn()),
  reconcileNow: vi.fn(async () => undefined),
  scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
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
}))

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
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.readMorrowToken.mockClear()
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
