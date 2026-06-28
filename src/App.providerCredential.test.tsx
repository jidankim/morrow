import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"
import type { MorrowTokenReadResponse } from "./tauriBridge"

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
  storeMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: true, deleted: false })),
  readMorrowToken: vi.fn(async (): Promise<MorrowTokenReadResponse> => ({
    storageSurface: "keychainBridge",
    present: true,
    token: "redacted-provider-token-for-ui"
  })),
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
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.deleteMorrowToken.mockClear()
  })

  it("saves checks and deletes provider credential without rendering the secret", async () => {
    renderSettings()
    await waitFor(() => expect(bridgeMock.readMorrowToken).toHaveBeenCalledOnce())
    bridgeMock.readMorrowToken.mockClear()

    fireEvent.change(await screen.findByLabelText("API key"), {
      target: { value: "redacted-provider-token-for-ui" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Save key" }))

    await waitFor(() => expect(bridgeMock.storeMorrowToken).toHaveBeenCalledOnce())
    expect(bridgeMock.storeMorrowToken).toHaveBeenCalledWith({
      service: "com.morrow.desktop.token",
      tokenKind: "morrow-openai-provider-api-key",
      token: "redacted-provider-token-for-ui"
    })
    expect(screen.getByLabelText("API key")).toHaveValue("")
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")

    fireEvent.click(screen.getByRole("button", { name: "Check key" }))
    await waitFor(() => expect(bridgeMock.readMorrowToken).toHaveBeenCalledOnce())
    expect(bridgeMock.readMorrowToken).toHaveBeenCalledWith({
      service: "com.morrow.desktop.token",
      tokenKind: "morrow-openai-provider-api-key"
    })
    expect(screen.getByText("OpenAI API key is stored.")).toBeInTheDocument()
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")

    fireEvent.click(screen.getByRole("button", { name: "Delete key" }))
    await waitFor(() => expect(bridgeMock.deleteMorrowToken).toHaveBeenCalledOnce())
    expect(bridgeMock.deleteMorrowToken).toHaveBeenCalledWith({
      service: "com.morrow.desktop.token",
      tokenKind: "morrow-openai-provider-api-key"
    })
    expect(screen.getByText("No OpenAI API key is stored.")).toBeInTheDocument()
  })

  it("blocks Sync Now and links to Settings when the provider credential is missing", async () => {
    bridgeMock.readMorrowToken.mockResolvedValueOnce({
      storageSurface: "keychainBridge",
      present: false
    })
    seedReadyState()
    render(<App />)

    await waitFor(() => expect(bridgeMock.readMorrowToken).toHaveBeenCalledOnce())
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled())

    expect(screen.getByTestId("sync-state")).toHaveTextContent("Disabled")
    expect(screen.getByText("Provider credential")).toBeInTheDocument()
    expect(screen.getAllByText("Save an OpenAI API key in Settings before scanning.")).not.toEqual([])
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")

    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole("button", { name: "Configure provider" }))
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
  })
})
