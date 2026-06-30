import { describe, expect, it } from "vitest"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState, loadAppShellState } from "./appShell"
import {
  chatDiscoveryFromReport,
  parseMessagesDiscoveryReport,
  parseSyncScanRequest
} from "../messagesDiscoveryBridge"

describe("app shell privacy boundaries", () => {
  it("defaults local diagnostics off while remote telemetry stays disabled", () => {
    const initial = createDefaultAppShellState()

    expect(initial.config.telemetryEnabled).toBe(false)
    expect(initial.config.feedbackTextSnapshotsEnabled).toBe(false)
    expect(initial.config.localDiagnosticsEnabled).toBe(false)
    expect(initial.config.localDiagnosticsRetentionDays).toBe(30)
  })

  it("parses local diagnostics settings without enabling remote telemetry", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true,
            telemetryEnabled: false,
            localDiagnosticsEnabled: true,
            localDiagnosticsRetentionDays: 14
          },
          discovery: { status: "unverified", chats: [] },
          selectedChats: [],
          pendingProposalCount: 0
        })
      ]
    ])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.telemetryEnabled).toBe(false)
    expect(reloaded.config.localDiagnosticsEnabled).toBe(true)
    expect(reloaded.config.localDiagnosticsRetentionDays).toBe(14)
  })

  it("rejects persisted remote telemetry opt-in", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true,
            telemetryEnabled: true,
            localDiagnosticsEnabled: false,
            localDiagnosticsRetentionDays: 30
          },
          discovery: { status: "unverified", chats: [] },
          selectedChats: [],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })

  it("rejects raw Messages chat guid shaped selected ids in local storage", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true
          },
          discovery: { status: "unverified", chats: [] },
          selectedChats: [
            {
              id: "iMessage;-;+15555550103",
              label: "Chat alpha",
              participantCount: 1,
              participantIds: ["messages-participant-11111111111111111111111111111111"],
              latestActivityTimestamp: 1_783_000_000,
              backfillPromptEnabled: true
            }
          ],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })

  it("maps native display labels to UI labels while rejecting body preview fields", () => {
    const nativeChat = {
      chatId: "messages-chat-11111111111111111111111111111111",
      displayLabel: "Team planning",
      participantCount: 1,
      participantIds: ["messages-participant-11111111111111111111111111111111"],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const report = parseMessagesDiscoveryReport({ status: "ready", chats: [nativeChat] })
    const baseScanRequest = {
      selectedChatIds: [nativeChat.chatId],
      selectedChats: [
        {
          id: nativeChat.chatId,
          label: nativeChat.displayLabel,
          participantCount: nativeChat.participantCount,
          participantIds: nativeChat.participantIds,
          latestActivityTimestamp: nativeChat.latestActivityTimestamp
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: [nativeChat.chatId],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    expect(chatDiscoveryFromReport(report).chats[0]).toEqual({
      id: nativeChat.chatId,
      label: "Team planning",
      participantCount: 1,
      participantIds: nativeChat.participantIds,
      latestActivityTimestamp: 1_783_000_000
    })
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...nativeChat, latestMessageBody: "private clinic visit" }]
      })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({ ...baseScanRequest, latestMessagePreview: "private clinic visit" })
    ).toThrow()
  })

  it("defaults privacy controls to no telemetry and scrubbed crash logs", () => {
    const initial = createDefaultAppShellState()

    expect(initial.config.telemetryEnabled).toBe(false)
    expect(initial.config.crashLogExcerptsEnabled).toBe(false)
    expect(initial.config.sourceExcerptsEnabled).toBe(true)
  })

  it("keeps old persisted settings on no-telemetry defaults", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true
          },
          selectedChats: [],
          pendingProposalCount: 0
        })
      ]
    ])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.telemetryEnabled).toBe(false)
    expect(reloaded.config.crashLogExcerptsEnabled).toBe(false)
  })

  it("rejects malformed persisted state at the local configuration boundary", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "paused",
          config: {
            referenceTimezone: "Mars/Olympus",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true
          },
          selectedChats: [
            {
              id: "../private-messages",
              label: "Injected chat",
              backfillPromptEnabled: true
            }
          ],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })

  it("rejects private-shaped selected chat metadata in local storage", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true
          },
          discovery: { status: "unverified", chats: [] },
          selectedChats: [
            {
              id: "messages-chat-11111111111111111111111111111111",
              label: "Chat alpha",
              participantCount: 1,
              participantIds: ["person@example.com"],
              latestActivityTimestamp: 1_783_000_000,
              backfillPromptEnabled: true,
              phone: "+15551234567"
            }
          ],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })
})
