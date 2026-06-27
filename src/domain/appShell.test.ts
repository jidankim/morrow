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

const selectedChat = {
  id: "chat-alpha",
  label: "Chat alpha",
  backfillPromptEnabled: true
} as const

describe("app shell state", () => {
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
    const setupOnly = reduceAppShellState(setupNeeded, {
      type: "updateConfig",
      config: { ...setupNeeded.config, permissionsGranted: true }
    })
    const scanning = { ...setupOnly, selectedChats: [selectedChat] }
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

  it("requires setup completion and at least one selected chat before scanning", () => {
    const initial = createDefaultAppShellState()
    const setupOnly = reduceAppShellState(initial, {
      type: "updateConfig",
      config: { ...initial.config, permissionsGranted: true }
    })
    const ready = { ...setupOnly, selectedChats: [selectedChat] }

    expect(isOnboardingComplete(initial)).toBe(false)
    expect(getOnboardingWarnings(initial)).toContain("Select at least one chat before scanning.")
    expect(isSyncNowEnabled(setupOnly)).toBe(false)
    expect(isOnboardingComplete(ready)).toBe(true)
    expect(isSyncNowEnabled(ready)).toBe(true)
  })

  it("does not add demo chat selections when native discovery has no options", () => {
    const initial = createDefaultAppShellState()
    const selected = reduceAppShellState(initial, {
      type: "toggleSelectedChat",
      chatId: selectedChat.id
    })

    expect(selected.selectedChats).toEqual([])
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
})
