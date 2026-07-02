import { describe, expect, it } from "vitest"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState, loadAppShellState } from "./appShell"

describe("app shell persisted privacy boundary", () => {
  it("defaults privacy controls to no telemetry and scrubbed crash logs", () => {
    const initial = createDefaultAppShellState()

    expect(initial.config.telemetryEnabled).toBe(false)
    expect(initial.config.crashLogExcerptsEnabled).toBe(false)
    expect(initial.config.sourceExcerptsEnabled).toBe(true)
    expect(initial.config.feedbackTextSnapshotsEnabled).toBe(false)
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
    expect(reloaded.config.feedbackTextSnapshotsEnabled).toBe(false)
  })

  it("rejects malformed feedback text snapshot consent in persisted settings", () => {
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
            feedbackTextSnapshotsEnabled: -1,
            firstProposalGuidanceEnabled: true
          },
          selectedChats: [],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })

  it("rejects malformed local diagnostics retention in persisted settings", () => {
    for (const localDiagnosticsRetentionDays of [0, 366, "30"]) {
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
              localDiagnosticsEnabled: true,
              localDiagnosticsRetentionDays
            },
            selectedChats: [],
            pendingProposalCount: 0
          })
        ]
      ])

      expect(() => loadAppShellState(storage)).toThrow()
    }
  })

  it("rejects persisted ICS feed calendar source at the local configuration boundary", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "ics-feed",
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

    expect(() => loadAppShellState(storage)).toThrow()
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
