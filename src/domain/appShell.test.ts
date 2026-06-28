import { describe, expect, it } from "vitest"
import {
  APP_SHELL_STATE_KEY,
  createDefaultAppShellState,
  formatPendingProposalCount,
  getMenuModel,
  getOnboardingWarnings,
  isOnboardingComplete,
  isSyncNowEnabled,
  loadAppShellState,
  reduceAppShellState,
  saveAppShellState
} from "./appShell"

const discoveredChat = { id: "messages-chat-11111111111111111111111111111111", label: "Chat alpha", participantCount: 2, participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"], latestActivityTimestamp: 1_783_000_000 } as const
const selectedChat = { ...discoveredChat, backfillPromptEnabled: true } as const

describe("app shell state", () => {
  it("defaults unsupported browser timezones to UTC", () => {
    const initial = createDefaultAppShellState("America/Los_Angeles")

    expect(initial.config.referenceTimezone).toBe("UTC")
  })

  it("persists pause and resume state across reloads", () => {
    const storage = new Map<string, string>()
    const initial = createDefaultAppShellState()

    const paused = reduceAppShellState(initial, { type: "pause" })
    saveAppShellState(paused, storage)
    const reloadedPaused = loadAppShellState(storage)

    const resumed = reduceAppShellState(reloadedPaused, { type: "resume" })
    saveAppShellState(resumed, storage)
    const reloadedResumed = loadAppShellState(storage)

    expect(reloadedPaused.mode).toBe("paused")
    expect(reloadedResumed.mode).toBe("scanning")
  })

  it("renders setup-needed before scanning when onboarding is incomplete", () => {
    const setupNeeded = createDefaultAppShellState()
    const scanning = {
      ...setupNeeded,
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [selectedChat]
    } as const
    const paused = reduceAppShellState(scanning, { type: "pause" })
    const error = reduceAppShellState(paused, {
      type: "fail",
      message: "Full Disk Access is unavailable"
    })

    expect(getMenuModel(setupNeeded).statusLabel).toBe("Setup needed")
    expect(getMenuModel(scanning).statusLabel).toBe("Scanning")
    expect(getMenuModel(paused).statusLabel).toBe("Paused")
    expect(getMenuModel(error).statusLabel).toBe("Error")
    expect(getMenuModel(error).detail).toBe("Full Disk Access is unavailable")
  })

  it("disables Sync Now while paused", () => {
    const scanning = createDefaultAppShellState()
    const paused = reduceAppShellState(scanning, { type: "pause" })

    expect(isSyncNowEnabled(scanning)).toBe(false)
    expect(isSyncNowEnabled(paused)).toBe(false)
    expect(getMenuModel(paused).syncNowEnabled).toBe(false)
  })

  it("requires verified discovery and at least one selected chat before scanning", () => {
    const initial = createDefaultAppShellState()
    const ready = {
      ...initial,
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [selectedChat]
    } as const

    expect(isOnboardingComplete(initial)).toBe(false)
    expect(getOnboardingWarnings(initial)).toContain("Select at least one chat before scanning.")
    expect(getOnboardingWarnings(ready)).not.toContain("Complete required permissions before scanning.")
    expect(isOnboardingComplete(ready)).toBe(true)
    expect(isSyncNowEnabled(ready)).toBe(true)
  })

  it("loads old persisted permissionsGranted false payloads without blocking ready scanning", () => {
    const oldPersistedState = {
      ...createDefaultAppShellState(),
      config: { ...createDefaultAppShellState().config, permissionsGranted: false },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [selectedChat]
    } as const
    const storage = new Map<string, string>([
      [APP_SHELL_STATE_KEY, JSON.stringify(oldPersistedState)]
    ])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.permissionsGranted).toBe(false)
    expect(isOnboardingComplete(reloaded)).toBe(true)
    expect(isSyncNowEnabled(reloaded)).toBe(true)
  })

  it("does not add demo chat selections when native discovery has no options", () => {
    const initial = createDefaultAppShellState()
    const selected = reduceAppShellState(initial, {
      type: "toggleSelectedChat",
      chatId: selectedChat.id
    })

    expect(selected.selectedChats).toEqual([])
  })

  it("selects and persists discovered chat metadata across reloads", () => {
    const storage = new Map<string, string>()
    const initial = {
      ...createDefaultAppShellState(),
      discovery: { status: "ready", chats: [discoveredChat] }
    } as const

    const selected = reduceAppShellState(initial, {
      type: "toggleSelectedChat",
      chatId: discoveredChat.id
    })
    saveAppShellState(selected, storage)
    const reloaded = loadAppShellState(storage)

    expect(reloaded.selectedChats).toEqual([selectedChat])
    expect(storage.get(APP_SHELL_STATE_KEY)).toContain("messages-participant-11111111111111111111111111111111")
  })

  it("keeps onboarding incomplete when discovery is not ready despite stale selections", () => {
    const defaultState = createDefaultAppShellState()
    const staleSelectedState = {
      ...defaultState,
      selectedChats: [selectedChat]
    }
    const states = [
      staleSelectedState,
      { ...staleSelectedState, discovery: { status: "loading", chats: [] } },
      { ...staleSelectedState, discovery: { status: "permissionDenied", chats: [] } },
      { ...staleSelectedState, discovery: { status: "unavailable", chats: [] } },
      { ...staleSelectedState, discovery: { status: "empty", chats: [] } },
      {
        ...staleSelectedState,
        discovery: {
          status: "ready",
          chats: [{ ...discoveredChat, id: "messages-chat-22222222222222222222222222222222" }]
        }
      }
    ] as const

    for (const state of states) {
      expect(isOnboardingComplete(state)).toBe(false)
      expect(isSyncNowEnabled(state)).toBe(false)
    }
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

  it("renders pending proposal counts exactly until nine plus", () => {
    expect(formatPendingProposalCount(0)).toBe("0")
    expect(formatPendingProposalCount(8)).toBe("8")
    expect(formatPendingProposalCount(9)).toBe("9+")
    expect(formatPendingProposalCount(14)).toBe("9+")
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
