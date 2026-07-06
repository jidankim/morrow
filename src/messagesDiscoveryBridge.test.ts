import { afterEach, describe, expect, it, vi } from "vitest"
import { DEFAULT_LIST_REMINDER_PROFILE } from "./domain/appConfig"
import { parseSyncScanRequest, syncScanRequestFromState } from "./messagesDiscoveryBridge"

const selectedChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Team planning",
  participantCount: 1,
  participantIds: ["messages-participant-11111111111111111111111111111111"],
  latestActivityTimestamp: 1_783_000_000,
  backfillPromptEnabled: true
} as const

function syncScanRequestState(referenceTimezone: string) {
  return {
    config: {
      referenceTimezone,
      calendarSource: "apple-calendar",
      permissionsGranted: true,
      launchAtLogin: false,
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: true,
      firstProposalGuidanceEnabled: true,
      telemetryEnabled: false,
      crashLogExcerptsEnabled: false,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE
    },
    pendingProposalCount: 0,
    selectedChats: [selectedChat]
  } as const
}

describe("syncScanRequestFromState", () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it("captures the current reference timestamp for native scan windows", () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date("2026-07-02T13:50:00Z"))

    // When
    const request = syncScanRequestFromState({
      config: {
        referenceTimezone: "Asia/Seoul",
        calendarSource: "apple-calendar",
        permissionsGranted: true,
        launchAtLogin: false,
        sourceExcerptsEnabled: false,
        feedbackTextSnapshotsEnabled: true,
        firstProposalGuidanceEnabled: true,
        telemetryEnabled: false,
        crashLogExcerptsEnabled: false,
        localDiagnosticsEnabled: false,
        localDiagnosticsRetentionDays: 30,
        listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE
      },
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.referenceUnixSeconds).toBe(1_783_000_200)
    expect(request.feedbackTextSnapshotsEnabled).toBe(true)
    expect(request.listReminderProfile).toEqual(DEFAULT_LIST_REMINDER_PROFILE)
  })

  it("forwards enabled list reminder profile settings from app state", () => {
    // Given
    const listReminderProfile = {
      ...DEFAULT_LIST_REMINDER_PROFILE,
      enabled: true,
      routingMode: "profileBareQuantityLists",
      defaultDueMode: "nextLocalDayAtDefaultTime"
    } as const
    const state = {
      ...syncScanRequestState("Asia/Seoul"),
      config: {
        ...syncScanRequestState("Asia/Seoul").config,
        listReminderProfile
      }
    } as const

    // When
    const request = syncScanRequestFromState(state)

    // Then
    expect(request.listReminderProfile).toEqual(listReminderProfile)
  })

  it("forwards persisted local diagnostics scan settings", () => {
    // Given
    const config = {
      referenceTimezone: "Asia/Seoul",
      calendarSource: "apple-calendar",
      permissionsGranted: true,
      launchAtLogin: false,
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      firstProposalGuidanceEnabled: true,
      telemetryEnabled: false,
      crashLogExcerptsEnabled: false,
      localDiagnosticsEnabled: true,
      localDiagnosticsRetentionDays: 45,
      listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE
    } as const

    // When
    const request = syncScanRequestFromState({
      config,
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.localDiagnosticsEnabled).toBe(true)
    expect(request.localDiagnosticsRetentionDays).toBe(45)
    expect(config.telemetryEnabled).toBe(false)
    expect(config.crashLogExcerptsEnabled).toBe(false)
  })

  it("resolves system timezone preference before native scan requests", () => {
    // Given
    const state = syncScanRequestState("system")

    // When
    const request = syncScanRequestFromState(state, "America/Los_Angeles")

    // Then
    expect(request.referenceTimezone).toBe("America/Los_Angeles")
  })

  it("preserves explicit timezone preference for native scan requests", () => {
    // Given
    const state = syncScanRequestState("Asia/Tokyo")

    // When
    const request = syncScanRequestFromState(state, "America/Los_Angeles")

    // Then
    expect(request.referenceTimezone).toBe("Asia/Tokyo")
  })

  it("rejects missing or malformed feedback text snapshot consent", () => {
    // Given
    const baseRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
          participantCount: 1,
          participantIds: ["messages-participant-11111111111111111111111111111111"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(parseSyncScanRequest(baseRequest).feedbackTextSnapshotsEnabled).toBe(true)
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        feedbackTextSnapshotsEnabled: -1
      })
    ).toThrow()
    const { feedbackTextSnapshotsEnabled: _missing, ...missingConsent } = baseRequest
    expect(() => parseSyncScanRequest(missingConsent)).toThrow()
  })

  it("rejects preview fields from native scan requests", () => {
    // Given
    const baseRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
          participantCount: 1,
          participantIds: ["messages-participant-11111111111111111111111111111111"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(() =>
      parseSyncScanRequest({ ...baseRequest, previews: { [selectedChat.id]: "private clinic visit" } })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        selectedChats: [{ ...baseRequest.selectedChats[0], messagePreview: "private clinic visit" }]
      })
    ).toThrow()
  })

  it("rejects malformed list reminder profiles and private profile fields", () => {
    // Given
    const baseRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
          participantCount: 1,
          participantIds: ["messages-participant-11111111111111111111111111111111"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: DEFAULT_LIST_REMINDER_PROFILE,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(parseSyncScanRequest(baseRequest).listReminderProfile).toEqual(DEFAULT_LIST_REMINDER_PROFILE)
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        listReminderProfile: {
          ...DEFAULT_LIST_REMINDER_PROFILE,
          routingMode: "alwaysSendBareLists"
        }
      })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        listReminderProfile: {
          ...DEFAULT_LIST_REMINDER_PROFILE,
          defaultDueTime: "08:30"
        }
      })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        listReminderProfile: {
          ...DEFAULT_LIST_REMINDER_PROFILE,
          participantIds: ["messages-participant-11111111111111111111111111111111"]
        }
      })
    ).toThrow()
  })
})
