import { beforeEach, describe, expect, it, vi } from "vitest"
import { DEFAULT_LIST_REMINDER_PROFILE } from "./domain/appConfig"
import type { NativeAppShellState } from "./domain/appShell"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(async (): Promise<unknown> => ({ pendingProposalCount: 3 })),
  listen: vi.fn()
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: tauriMock.listen
}))

describe("createNativeShellBridge runtime identity", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__")
  })

  it("parses binary and appBundle runtime identity payloads", async () => {
    // Given
    const { parseRuntimeIdentity } = await import("./nativeRuntimeBridge")
    const binaryPayload = {
      displayName: "Morrow",
      bundleIdentifier: "dev.morrow.desktop",
      executablePath: "/Users/example/morrow/target/debug/morrow",
      settingsTargetPath: "/Users/example/morrow/target/debug/morrow",
      runtimeKind: "binary"
    }
    const appBundlePayload = {
      displayName: "Morrow",
      bundleIdentifier: "dev.morrow.desktop",
      executablePath: "/Applications/Morrow.app/Contents/MacOS/morrow",
      settingsTargetPath: "/Applications/Morrow.app",
      runtimeKind: "appBundle"
    }

    // When
    const binaryIdentity = parseRuntimeIdentity(binaryPayload)
    const appBundleIdentity = parseRuntimeIdentity(appBundlePayload)

    // Then
    expect(binaryIdentity).toEqual(binaryPayload)
    expect(appBundleIdentity).toEqual(appBundlePayload)
  })

  it("rejects malformed runtime identity payloads", async () => {
    // Given
    const { parseRuntimeIdentity } = await import("./nativeRuntimeBridge")
    const malformedRuntimeKind = {
      displayName: "Morrow",
      bundleIdentifier: "dev.morrow.desktop",
      executablePath: "/Users/example/morrow/target/release/morrow",
      settingsTargetPath: "/Users/example/morrow/target/release/morrow",
      runtimeKind: "releaseBinary"
    }
    const emptySettingsTargetPath = {
      displayName: "Morrow",
      bundleIdentifier: "dev.morrow.desktop",
      executablePath: "/Users/example/morrow/target/release/morrow",
      settingsTargetPath: "",
      runtimeKind: "binary"
    }

    // When / Then
    expect(() => parseRuntimeIdentity(malformedRuntimeKind)).toThrow()
    expect(() => parseRuntimeIdentity(emptySettingsTargetPath)).toThrow()
  })

  it("invokes native get_runtime_identity in Tauri runtime", async () => {
    // Given
    const payload = {
      displayName: "Morrow",
      bundleIdentifier: "dev.morrow.desktop",
      executablePath: "/Users/example/morrow/target/debug/morrow",
      settingsTargetPath: "/Users/example/morrow/target/debug/morrow",
      runtimeKind: "binary"
    } as const
    tauriMock.invoke.mockResolvedValueOnce(payload)
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const identity = await bridge.getRuntimeIdentity()

    // Then
    expect(identity).toEqual(payload)
    expect(tauriMock.invoke).toHaveBeenCalledWith("get_runtime_identity")
  })

  it("returns undefined fallbacks outside Tauri runtime without native invocations", async () => {
    // Given
    const { createNativeShellBridge, MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const shellState: NativeAppShellState = {
      mode: "scanning",
      onboardingComplete: true,
      pendingProposalCount: 0,
      syncNowRunning: false,
      automaticSyncEnabled: false,
      automaticSyncStatusLabel: "Off",
      automaticSyncDetail: "Automatic sync is off."
    }
    const selectedChat = { id: "messages-chat-11111111111111111111111111111111", label: "Team planning", participantCount: 1, participantIds: ["messages-participant-11111111111111111111111111111111"], latestActivityTimestamp: 1 } as const
    const scanRequest = { selectedChatIds: [selectedChat.id], selectedChats: [selectedChat], referenceTimezone: "Asia/Seoul", referenceUnixSeconds: 1, backfillPromptChatIds: [selectedChat.id], sourceExcerptsEnabled: false, feedbackTextSnapshotsEnabled: false, localDiagnosticsEnabled: false, localDiagnosticsRetentionDays: 30, listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE, capPolicy: { mode: "refillForPending", maxVisible: 1, pendingCount: 0 } } as const
    const previewRequest = { chatIds: [selectedChat.id] } as const
    const tokenLookup = { service: MORROW_KEYCHAIN_SERVICE, tokenKind: MORROW_PROVIDER_TOKEN_KIND } as const
    const deleteDataRequest = { confirmation: "DELETE MORROW DATA", cleanupProposedItems: true, deleteEmptyProposalContainers: true, revokeProviderOAuth: true } as const
    const privacySettingsRequest = { pane: "calendar" } as const
    const crashLogRequest = { message: "fallback characterization" } as const

    // When / Then
    const fallbackResults = await Promise.all([bridge.getState(), bridge.setShellState(shellState), bridge.getRuntimeIdentity(), bridge.reconcileNow(), bridge.scanSelectedChats(scanRequest), bridge.discoverMessagesChats(), bridge.loadMessagesChatPreviews(previewRequest), bridge.getPermissionStatuses(), bridge.storeMorrowToken({ ...tokenLookup, token: "redacted-token" }), bridge.readMorrowToken(tokenLookup), bridge.deleteMorrowToken(tokenLookup), bridge.checkProviderAuth(), bridge.deleteMorrowData(deleteDataRequest), bridge.openPrivacySettings(privacySettingsRequest), bridge.recordCrashLog(crashLogRequest), bridge.subscribeAppState(() => undefined), bridge.subscribeMenuCommand(() => undefined)])
    expect(fallbackResults).toEqual(Array.from({ length: 17 }, () => undefined))
    expect(tauriMock.invoke).not.toHaveBeenCalled()
    expect(tauriMock.listen).not.toHaveBeenCalled()
  })

  it("returns undefined for messages calls outside Tauri before invoking or parsing payloads", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const malformedSelectedChat = { id: "+15555550103", label: "Team planning", participantCount: 1, participantIds: ["messages-participant-11111111111111111111111111111111"], latestActivityTimestamp: 1 } as const
    const malformedScanRequest = { selectedChatIds: [malformedSelectedChat.id], selectedChats: [malformedSelectedChat], referenceTimezone: "Asia/Seoul", referenceUnixSeconds: 1, backfillPromptChatIds: [malformedSelectedChat.id], sourceExcerptsEnabled: false, feedbackTextSnapshotsEnabled: false, localDiagnosticsEnabled: false, localDiagnosticsRetentionDays: 30, listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE, capPolicy: { mode: "refillForPending", maxVisible: 1, pendingCount: 0 } } as const
    const malformedPreviewRequest = { chatIds: [malformedSelectedChat.id] } as const

    // When / Then
    await expect(bridge.scanSelectedChats(malformedScanRequest)).resolves.toBeUndefined()
    await expect(bridge.discoverMessagesChats()).resolves.toBeUndefined()
    await expect(bridge.loadMessagesChatPreviews(malformedPreviewRequest)).resolves.toBeUndefined()
    expect(tauriMock.invoke).not.toHaveBeenCalled()
    expect(tauriMock.listen).not.toHaveBeenCalled()
  })
})
