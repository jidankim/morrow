import { render, screen, within } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { StatusView } from "./StatusView"
import {
  createDefaultAppShellState,
  getMenuModel,
  getOnboardingWarnings,
  isSyncNowEnabled,
  type AppShellState,
  type ChatId,
  type DiscoveredChat,
  type SelectedChat
} from "./domain/appShell"
import { decisionEvidenceReportSchema } from "./domain/decisionEvidence"
import { createDefaultSyncSchedulerState } from "./domain/syncScheduler"

const nowUnixSeconds = 1_783_000_000

describe("StatusView decision evidence", () => {
  it("renders candidate and quiet decision evidence with trace retention states", () => {
    // Given
    const candidateId = "candidate-alpha-opaque-id-with-long-suffix-0123456789abcdef"
    const state = readyState({
      decisionEvidence: {
        items: [
          {
            subjectType: "candidate",
            candidateId,
            candidateState: "draft",
            candidateKind: "calendar_event",
            route: "provider",
            reasonCode: "accepted_for_calendar",
            confidenceMillis: 842,
            labelType: "candidate",
            labelValue: "created",
            sourceExcerptPolicy: "disabled",
            privacyTier: "safe",
            hasDiagnosticsHashes: true,
            createdAt: nowUnixSeconds,
            traceRetention: "retained",
            traceSequence: [
              {
                component: "scan",
                operation: "classify",
                decision: "candidate",
                outcome: "accepted"
              },
              {
                component: "proposal",
                operation: "create",
                outcome: "queued"
              }
            ]
          },
          {
            subjectType: "quietLog",
            route: "provider",
            reasonCode: "provider_unavailable",
            providerDiagnostic: "codex provider command timed out",
            labelType: "quiet",
            labelValue: "rejected",
            sourceExcerptPolicy: "disabled",
            privacyTier: "safe",
            hasDiagnosticsHashes: false,
            createdAt: nowUnixSeconds - 30,
            traceRetention: "traceMissing",
            traceSequence: [
              {
                component: "scan",
                operation: "classify",
                decision: "quiet",
                outcome: "not actionable"
              }
            ]
          }
        ],
        skippedTraceLineCount: 0,
        latestEvalStatus: "needs_review"
      }
    })

    // When
    renderStatusView({ state })

    // Then
    expect(screen.getByRole("heading", { name: "Decision evidence" })).toBeInTheDocument()
    expect(screen.getByRole("list", { name: "Recent decision evidence" })).toBeInTheDocument()
    expect(screen.getByText("Candidate")).toBeInTheDocument()
    expect(screen.getByText("Quiet")).toBeInTheDocument()
    expect(screen.getByText("Trace retained")).toBeInTheDocument()
    expect(screen.getByText("Trace not retained")).toBeInTheDocument()
    expect(screen.getByText("provider / accepted_for_calendar")).toBeInTheDocument()
    expect(screen.getByText("provider / provider_unavailable")).toBeInTheDocument()
    expect(screen.getByText("codex provider command timed out")).toBeInTheDocument()
    expect(screen.getByText("Confidence 842 ms")).toBeInTheDocument()
    expect(screen.getByLabelText(`Candidate ID ${candidateId}`)).toHaveTextContent(`ID${candidateId}`)
    expect(screen.getByText("candidate: created · Eval needs review")).toBeInTheDocument()
    expect(screen.getByText("quiet: rejected · Eval needs review")).toBeInTheDocument()
    expect(screen.getByText("scan classify candidate accepted -> proposal create queued")).toBeInTheDocument()
    expect(screen.getByText("scan classify quiet not actionable")).toBeInTheDocument()
    const quietRow = screen.getByText("Quiet").closest("li")
    if (quietRow === null) {
      throw new Error("Quiet decision evidence row was not rendered")
    }
    expect(within(quietRow).queryByLabelText(/Candidate ID/u)).not.toBeInTheDocument()
  })

  it("normalizes optional provider diagnostics at the native schema boundary", () => {
    // Given
    const reportWithDiagnostic = decisionEvidenceReport({
      providerDiagnostic: "codex provider command timed out"
    })
    const reportWithNullDiagnostic = decisionEvidenceReport({ providerDiagnostic: null })
    const reportWithoutDiagnostic = decisionEvidenceReport({})
    const reportWithEmptyDiagnostic = decisionEvidenceReport({ providerDiagnostic: "" })

    // When
    const parsedWithDiagnostic = decisionEvidenceReportSchema.parse(reportWithDiagnostic)
    const parsedWithNullDiagnostic = decisionEvidenceReportSchema.parse(reportWithNullDiagnostic)
    const parsedWithoutDiagnostic = decisionEvidenceReportSchema.parse(reportWithoutDiagnostic)

    // Then
    expect(parsedWithDiagnostic.items[0]?.providerDiagnostic).toBe("codex provider command timed out")
    expect(parsedWithNullDiagnostic.items[0]?.providerDiagnostic).toBeUndefined()
    expect(parsedWithoutDiagnostic.items[0]?.providerDiagnostic).toBeUndefined()
    expect(() => decisionEvidenceReportSchema.parse(reportWithEmptyDiagnostic)).toThrow()
  })

  it("renders compact empty and loading states without raw private field names", () => {
    // Given
    const emptyState = readyState()
    const forbiddenTokens = [
      "rawText",
      "prompt",
      "response",
      "rawJson",
      "providerJson",
      "fullMessage",
      "rawTitle",
      "titleText",
      "unredactedTitle",
      "chat_guid",
      "anchor_message_guid",
      "diagnosticsPath",
      "appDataDir",
      "traceId",
      "spanId"
    ] as const

    // When
    const { rerender } = renderStatusView({ state: emptyState })

    // Then
    expect(screen.getByText("No recent decision evidence.")).toBeInTheDocument()
    rerender(statusViewElement({ state: emptyState, syncing: true }))
    expect(screen.getByRole("status", { name: "Decision evidence loading" })).toHaveTextContent(
      "Loading decision evidence."
    )
    for (const token of forbiddenTokens) {
      expect(document.body).not.toHaveTextContent(token)
    }
  })
})

function renderStatusView({
  state,
  syncing = false
}: {
  readonly state: AppShellState
  readonly syncing?: boolean
}) {
  return render(statusViewElement({ state, syncing }))
}

function statusViewElement({
  state,
  syncing = false
}: {
  readonly state: AppShellState
  readonly syncing?: boolean
}): JSX.Element {
  return (
    <StatusView
      menu={getMenuModel(state)}
      state={state}
      warnings={getOnboardingWarnings(state)}
      syncing={syncing}
      syncEnabled={isSyncNowEnabled(state)}
      syncScheduler={createDefaultSyncSchedulerState(nowUnixSeconds)}
      syncSchedulerNowUnixSeconds={nowUnixSeconds}
      onChangeAutomaticSyncInterval={noop}
      onHidePreviews={noop}
      onOpenFullDiskAccess={noop}
      onOpenSettings={noop}
      onPause={noop}
      onResume={noop}
      onRetryChatDiscovery={noop}
      onRevealPreviews={noop}
      onSyncNow={noop}
      onToggleAutomaticSync={noop}
      onToggleBackfillPrompt={noopBackfillToggle}
      onToggleChat={noopChatToggle}
    />
  )
}

function readyState(overrides: Partial<AppShellState> = {}): AppShellState {
  const base = createDefaultAppShellState("Asia/Seoul")
  return {
    ...base,
    ...overrides,
    config: { ...base.config, permissionsGranted: true, ...overrides.config },
    discovery: { status: "ready", chats: [chat] },
    providerCredentialStatus: "configured",
    selectedChats: [selectedChat]
  }
}

function decisionEvidenceReport({
  providerDiagnostic
}: {
  readonly providerDiagnostic?: string | null | undefined
}) {
  const item =
    providerDiagnostic === undefined
      ? decisionEvidenceItem()
      : decisionEvidenceItem({ providerDiagnostic })
  return {
    items: [item],
    skippedTraceLineCount: 0,
    latestEvalStatus: "needs_review"
  }
}

function decisionEvidenceItem(extraFields: Record<string, string | null> = {}) {
  return {
    subjectType: "quietLog",
    route: "provider",
    reasonCode: "provider_unavailable",
    labelType: "quiet",
    labelValue: "rejected",
    sourceExcerptPolicy: "disabled",
    privacyTier: "safe",
    hasDiagnosticsHashes: false,
    createdAt: nowUnixSeconds - 30,
    traceRetention: "traceMissing",
    traceSequence: [
      {
        component: "scan",
        operation: "classify",
        decision: "quiet",
        outcome: "not actionable"
      }
    ],
    ...extraFields
  }
}

const chat: DiscoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Planning circle",
  latestActivityTimestamp: nowUnixSeconds,
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ]
}

const selectedChat: SelectedChat = { ...chat, backfillPromptEnabled: true }

function noop(): void {}

function noopChatToggle(_chatId: ChatId): void {}

function noopBackfillToggle(_chatId: ChatId, _enabled: boolean): void {}
