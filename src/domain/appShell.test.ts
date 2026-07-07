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
import {
  isReferenceTimeZonePreference,
  referenceTimeZoneOptions,
  resolveReferenceTimeZonePreference
} from "./timeZone"

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
const selectedChat = { ...discoveredChat, backfillPromptEnabled: true } as const

describe("app shell state", () => {
  it("defaults to the system reference timezone preference", () => {
    const initial = createDefaultAppShellState("America/Los_Angeles")

    expect(initial.config.referenceTimezone).toBe("system")
  })

  it("resolves the system reference timezone preference to a concrete timezone", () => {
    const resolved = resolveReferenceTimeZonePreference("system", "America/Los_Angeles")

    expect(resolved).toBe("America/Los_Angeles")
  })

  it("falls back to UTC when the system reference timezone is unavailable or invalid", () => {
    expect(resolveReferenceTimeZonePreference("system", undefined)).toBe("UTC")
    expect(resolveReferenceTimeZonePreference("system", "")).toBe("UTC")
    expect(resolveReferenceTimeZonePreference("system", "Mars/Olympus")).toBe("UTC")
  })

  it("recognizes system and concrete reference timezone preferences", () => {
    expect(isReferenceTimeZonePreference("system")).toBe(true)
    expect(isReferenceTimeZonePreference("America/New_York")).toBe(true)
    expect(isReferenceTimeZonePreference("")).toBe(false)
    expect(isReferenceTimeZonePreference("Mars/Olympus")).toBe(false)
  })

  it("builds reference timezone options with system default first", () => {
    const options = referenceTimeZoneOptions("America/Los_Angeles")

    expect(options[0]).toEqual({
      value: "system",
      label: "System default (America/Los_Angeles)"
    })
    expect(options).toContainEqual({
      value: "America/Los_Angeles",
      label: "America/Los_Angeles"
    })
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

  it("renders setup-needed before ready when onboarding is incomplete", () => {
    const setupNeeded = createDefaultAppShellState()
    const scanning = {
      ...setupNeeded,
      providerCredentialStatus: "configured",
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [selectedChat]
    } as const
    const paused = reduceAppShellState(scanning, { type: "pause" })
    const error = reduceAppShellState(paused, {
      type: "fail",
      message: "Full Disk Access is unavailable"
    })

    expect(getMenuModel(setupNeeded).statusLabel).toBe("Setup needed")
    expect(getMenuModel(scanning).statusLabel).toBe("Ready")
    expect(getMenuModel(scanning).detail).toBe(
      "Ready. Use Sync Now to reconcile calendars and scan selected chats."
    )
    expect(getMenuModel(scanning).pauseResumeLabel).toBe("Disable Sync Now")
    expect(getMenuModel(paused).statusLabel).toBe("Sync Now disabled")
    expect(getMenuModel(paused).detail).toBe("Sync Now is disabled on this Mac.")
    expect(getMenuModel(paused).pauseResumeLabel).toBe("Enable Sync Now")
    expect(getMenuModel(error).statusLabel).toBe("Error")
    expect(getMenuModel(error).detail).toBe("Full Disk Access is unavailable")
    expect(getMenuModel(error).pauseResumeLabel).toBe("Disable Sync Now")
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
      providerCredentialStatus: "configured",
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
      providerCredentialStatus: "configured",
      config: { referenceTimezone: "system", calendarSource: "apple-calendar", permissionsGranted: false, launchAtLogin: false, sourceExcerptsEnabled: true, firstProposalGuidanceEnabled: true },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [selectedChat]
    } as const
    const storage = new Map<string, string>([[APP_SHELL_STATE_KEY, JSON.stringify(oldPersistedState)]])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.permissionsGranted).toBe(false)
    expect(reloaded.config.listIntakeProfiles).toEqual([])
    expect(reloaded.providerCredentialStatus).toBe("unchecked")
    expect(isOnboardingComplete(reloaded)).toBe(false)
    expect(isSyncNowEnabled(reloaded)).toBe(false)
  })

  it("loads persisted null error messages as absent", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({ ...createDefaultAppShellState(), errorMessage: null })
      ]
    ])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.errorMessage).toBeUndefined()
  })

  it("loads persisted error state as normal startup state", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          ...createDefaultAppShellState(),
          mode: "error",
          errorMessage: "Expected string, received null"
        })
      ]
    ])

    const reloaded = loadAppShellState(storage)

    expect(reloaded.mode).toBe("scanning")
    expect(reloaded.errorMessage).toBeUndefined()
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

  it("reloads a persisted concrete reference timezone without converting it to system", () => {
    const storage = new Map<string, string>()
    const initial = {
      ...createDefaultAppShellState("America/Los_Angeles"),
      config: {
        ...createDefaultAppShellState("America/Los_Angeles").config,
        referenceTimezone: "America/New_York"
      }
    } as const

    saveAppShellState(initial, storage)
    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.referenceTimezone).toBe("America/New_York")
  })

  it("reloads a persisted system reference timezone preference", () => {
    const storage = new Map<string, string>()
    const initial = createDefaultAppShellState("America/Los_Angeles")

    saveAppShellState(initial, storage)
    const reloaded = loadAppShellState(storage)

    expect(reloaded.config.referenceTimezone).toBe("system")
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

  it("renders pending proposal counts exactly until nine plus", () => {
    expect(formatPendingProposalCount(0)).toBe("0")
    expect(formatPendingProposalCount(8)).toBe("8")
    expect(formatPendingProposalCount(9)).toBe("9+")
    expect(formatPendingProposalCount(14)).toBe("9+")
  })

})
