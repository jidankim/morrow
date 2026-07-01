import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"
import type { SyncSchedulerState } from "./domain/syncScheduler"

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
    subscribeMenuCommand: vi.fn(async () => vi.fn()),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
    getSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
    setSyncSchedulerState: vi.fn(async (state: SyncSchedulerState) => state),
    storeMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: true,
      deleted: false
    })),
    checkProviderAuth: vi.fn(async () => ({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
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
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

const renderSettings = (): void => {
  render(<App />)
  act(() => {
    window.location.hash = "#settings"
    window.dispatchEvent(new HashChangeEvent("hashchange"))
  })
}

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

describe("App provider credential controls", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.storeMorrowToken.mockClear()
    bridgeMock.checkProviderAuth.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.deleteMorrowToken.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
  })

  it("shows Codex provider readiness without an API-key password field", async () => {
    renderSettings()

    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    expect(screen.queryByText("OpenAI API key")).not.toBeInTheDocument()
    expect(screen.queryByLabelText("API key")).not.toBeInTheDocument()
    expect(document.querySelector('input[type="password"]')).toBeNull()
    expect(screen.getByRole("heading", { name: "Codex provider" })).toBeInTheDocument()
    expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
    expect(bridgeMock.storeMorrowToken).not.toHaveBeenCalled()
    expect(bridgeMock.readMorrowToken).not.toHaveBeenCalled()
    expect(bridgeMock.deleteMorrowToken).not.toHaveBeenCalled()
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")

    fireEvent.click(screen.getByRole("button", { name: "Refresh readiness" }))
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(2))
  })

  it("blocks Sync Now and links to Settings when Codex provider readiness is missing", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "notLoggedIn",
      ready: false,
      commandSurface: "codex login status --raw codex_access_token=leaked",
      commandOutputRedacted: true,
      diagnostic: "raw codex_access_token=leaked"
    })
    seedReadyState()
    render(<App />)

    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled())

    expect(screen.getByTestId("sync-state")).toHaveTextContent("Disabled")
    expect(screen.getByText("Codex provider")).toBeInTheDocument()
    expect(screen.getAllByText("Finish Codex CLI setup in Settings before scanning.")).not.toEqual([])
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")
    expect(document.body).not.toHaveTextContent("codex_access_token=leaked")
    expect(document.body).not.toHaveTextContent("codex login status --raw")

    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole("button", { name: "Configure provider" }))
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
  })

  it("copies the static Codex login command without showing provider output", async () => {
    const writeText = vi.fn(async () => undefined)
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText }
    })
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "missingCli",
      ready: false,
      commandSurface: "codex login status with secret raw output",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Copy login command" }))

    await waitFor(() => expect(writeText).toHaveBeenCalledWith("codex login"))
    expect(screen.getByText("Login command copied.")).toBeInTheDocument()
    expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
    expect(document.body).not.toHaveTextContent("secret raw output")
  })
})
