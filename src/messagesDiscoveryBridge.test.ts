import { afterEach, describe, expect, it, vi } from "vitest"
import { createListIntakeProfile } from "./domain/appConfig"
import { parseSyncScanRequest, syncScanRequestFromState } from "./messagesDiscoveryBridge"

const selectedChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Team planning",
  participantCount: 1,
  participantIds: ["messages-participant-11111111111111111111111111111111"],
  latestActivityTimestamp: 1_783_000_000,
  backfillPromptEnabled: true
} as const
const validListIntakeProfile = createListIntakeProfile({
  enabled: true,
  profileId: "list-intake-fishcount",
  name: "Fish count",
  profileVersion: "list-intake-v2",
  kind: "quantityList",
  extractionMode: "providerConstrained",
  providerPromptVersion: "list-intake-v1",
  positiveExamples: ["2 anchovies, 3 salmon"],
  negativeExamples: ["remind me to buy fish tomorrow"],
  categoryRules: [{ categoryId: "seafood", displayName: "Seafood", keywords: ["salmon"] }],
  aggregation: { window: "localDay", timezoneSource: "referenceTimezone" },
  chatScope: { mode: "selectedChatIds", selectedChatIds: [selectedChat.id] },
  grouping: { chat: true, sender: "displayAlias" },
  captureFromScheduledMessages: false,
  outputPolicy: "dailyDigestReminder",
  digestReminder: { dueTimeLocal: "09:00", dateOffsetDays: 1, outputPolicyVersion: "list-intake-digest-v1" },
  quantityListBounds: {
    minItems: 1,
    maxItems: 20,
    minQuantity: 1,
    maxQuantity: 999,
    maxItemNameVisibleChars: 80,
    maxUnitVisibleChars: 24,
    uncategorizedCategoryId: "uncategorized"
  },
  thresholds: { autoAggregateThresholdMillis: 850, reviewThresholdMillis: 550 }
})

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
      listIntakeProfiles: []
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
        listIntakeProfiles: []
      },
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.referenceUnixSeconds).toBe(1_783_000_200)
    expect(request.feedbackTextSnapshotsEnabled).toBe(true)
    expect(request.listIntakeProfiles).toEqual([])
  })

  it("forwards explicit list-intake profiles from app state without legacy scan fields", () => {
    // Given
    const state = {
      ...syncScanRequestState("Asia/Seoul"),
      config: {
        ...syncScanRequestState("Asia/Seoul").config,
        listIntakeProfiles: [validListIntakeProfile]
      }
    } as const

    // When
    const request = syncScanRequestFromState(state)

    // Then
    expect(request.listIntakeProfiles).toEqual([validListIntakeProfile])
    expect(request).not.toHaveProperty("listReminderProfile")
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
      listIntakeProfiles: []
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
      listIntakeProfiles: [validListIntakeProfile],
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
      listIntakeProfiles: [validListIntakeProfile],
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

})
