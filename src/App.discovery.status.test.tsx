import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { ChatDiscoveryControls } from "./ChatDiscoveryControls"
import type { ChatPreviewDisclosure } from "./ChatPreviewControls"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"
import type { SyncSchedulerState } from "./domain/syncScheduler"

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

type NativeReadyChatForTest = Extract<NativeDiscoveryReportForTest, { readonly status: "ready" }>["chats"][number]
type ChatFixtureForTest = Pick<NativeReadyChatForTest, "participantCount" | "participantIds" | "latestActivityTimestamp"> & {
  readonly id: string
  readonly label: string
}

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

const nativeChatFromFixture = (chat: ChatFixtureForTest): NativeReadyChatForTest => ({
  chatId: chat.id,
  displayLabel: chat.label,
  participantCount: chat.participantCount,
  participantIds: chat.participantIds,
  latestActivityTimestamp: chat.latestActivityTimestamp
})

const nativeReadyReport = { status: "ready", chats: [nativeChatFromFixture(discoveredChat)] } as const

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
    loadDecisionEvidence: vi.fn(async () => ({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "never_run"
    })),
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
    discoverMessagesChats: vi.fn(async (): Promise<NativeDiscoveryReportForTest> => nativeReadyReport),
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

const seedPermissionsGranted = (): void => {
  const initial = createDefaultAppShellState()
  const ready = { ...initial, config: { ...initial.config, permissionsGranted: true } }
  window.localStorage.setItem(APP_SHELL_STATE_KEY, JSON.stringify(ready))
}

const idleDiscoveryControlProps = {
  referenceTimezone: "UTC",
  selectedChats: [],
  onOpenFullDiskAccess: () => undefined,
  onRetry: () => undefined,
  onToggleBackfillPrompt: () => undefined,
  onToggleChat: () => undefined
}

const hiddenPreviewDisclosure: ChatPreviewDisclosure = {
  status: "hidden",
  previews: new Map()
}

describe("App Messages chat discovery status and recovery", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.setShellState.mockClear()
    bridgeMock.reconcileNow.mockClear()
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
    bridgeMock.openPrivacySettings.mockClear()
  })

  it("renders distinct empty discovery copy", async () => {
    let resolveDiscovery: (report: NativeDiscoveryReportForTest) => void = () => undefined
    bridgeMock.discoverMessagesChats.mockReturnValueOnce(
      new Promise<NativeDiscoveryReportForTest>((resolve) => {
        resolveDiscovery = resolve
      })
    )
    render(<App />)

    expect((await screen.findAllByText("Morrow is checking local Messages access.")).length).toBeGreaterThan(1)
    await act(async () => resolveDiscovery({ status: "empty", chats: [] }))

    expect(screen.getByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    expect(screen.getByText("Messages discovery found no eligible chats.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
  })

  it("shows Full Disk Access recovery when Messages discovery is denied", async () => {
    seedPermissionsGranted()
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    render(<App />)

    expect(await screen.findByText("Full Disk Access recovery")).toBeInTheDocument()
    expect(screen.getByText("Sync Now disabled: Grant Full Disk Access, restart Morrow, then retry chat discovery.")).toBeInTheDocument()
    expect(screen.getAllByText("Enable or add the app that launched Morrow in Full Disk Access.").length).toBeGreaterThan(0)
    expect(screen.getAllByText("After changing Full Disk Access, restart Morrow, then retry chat discovery.").length).toBeGreaterThan(0)
    expect(screen.getByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
  })

  it("renders distinct denied discovery recovery copy", async () => {
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    render(<App />)

    expect(await screen.findByText("Full Disk Access recovery")).toBeInTheDocument()
    expect(screen.getAllByText("Enable or add the app that launched Morrow in Full Disk Access.").length).toBeGreaterThan(0)
    expect(screen.getAllByText("After changing Full Disk Access, restart Morrow, then retry chat discovery.").length).toBeGreaterThan(0)
    fireEvent.click(screen.getByRole("button", { name: "Open Full Disk Access" }))

    await waitFor(() => expect(bridgeMock.openPrivacySettings).toHaveBeenCalledWith({ pane: "fullDiskAccess" }))
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
  })

  it("retries discovery from unavailable and empty states", async () => {
    bridgeMock.discoverMessagesChats
      .mockResolvedValueOnce({ status: "unavailable", chats: [] })
      .mockResolvedValueOnce({ status: "empty", chats: [] })
      .mockResolvedValueOnce(nativeReadyReport)
    render(<App />)

    expect(await screen.findByText("Full Disk Access recovery")).toBeInTheDocument()
    expect(screen.getAllByText("Enable or add the app that launched Morrow in Full Disk Access.").length).toBeGreaterThan(0)
    expect(screen.getAllByText("After changing Full Disk Access, restart Morrow, then retry chat discovery.").length).toBeGreaterThan(0)
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))
    expect(await screen.findByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(bridgeMock.discoverMessagesChats).toHaveBeenCalledTimes(3)
  })

  it("renders distinct unverified discovery copy", () => {
    render(<ChatDiscoveryControls {...idleDiscoveryControlProps} discovery={{ status: "unverified", chats: [] }} />)

    expect(screen.getByText("Messages discovery has not completed yet.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Retry chat discovery" })).toBeInTheDocument()
  })

  it("shows preview reveal controls only for ready discovery", () => {
    const { rerender } = render(
      <ChatDiscoveryControls
        {...idleDiscoveryControlProps}
        discovery={{ status: "unverified", chats: [] }}
        previewDisclosure={hiddenPreviewDisclosure}
        onHidePreviews={() => undefined}
        onRevealPreviews={() => undefined}
      />
    )

    expect(screen.queryByRole("button", { name: "Reveal previews locally" })).not.toBeInTheDocument()

    rerender(
      <ChatDiscoveryControls
        {...idleDiscoveryControlProps}
        discovery={{ status: "ready", chats: [discoveredChat] }}
        previewDisclosure={hiddenPreviewDisclosure}
        onHidePreviews={() => undefined}
        onRevealPreviews={() => undefined}
      />
    )

    expect(screen.getByRole("button", { name: "Reveal previews locally" })).toBeInTheDocument()
  })
})
