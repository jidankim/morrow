import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { useRef, useState } from "react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import {
  createDefaultAppShellState,
  type AppShellState,
  type NativeAppShellState
} from "./domain/appShell"
import type { MorrowDataDeleteReceipt, MorrowDataDeleteRequest } from "./nativePrivacyBridge"
import type { NativeMenuCommand, PrivacySettingsReceipt, SyncScanResult } from "./tauriBridge"
import type { SyncSchedulerState } from "./syncSchedulerTauriBridge"
import { useSyncSchedulerOrchestrator } from "./useSyncSchedulerOrchestrator"

const selectedChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Planning",
  participantCount: 1,
  participantIds: ["messages-participant-11111111111111111111111111111111"],
  latestActivityTimestamp: 1_783_000_000,
  backfillPromptEnabled: false
} as const

const decisionEvidenceReport = {
  items: [
    {
      subjectType: "candidate",
      candidateId: "candidate-alpha",
      candidateState: "draft",
      candidateKind: "calendar_event",
      route: "provider",
      reasonCode: "accepted",
      confidenceMillis: 830,
      labelType: "candidate",
      labelValue: "created",
      sourceExcerptPolicy: "disabled",
      privacyTier: "safe",
      hasDiagnosticsHashes: true,
      createdAt: 1_783_000_010,
      traceRetention: "retained",
      traceSequence: [
        {
          component: "provider",
          operation: "scan",
          decision: "accept",
          outcome: "created"
        }
      ]
    }
  ],
  skippedTraceLineCount: 0,
  latestEvalStatus: "passed"
} as const

function readyStateWithDecisionEvidence(): AppShellState {
  return {
    ...createDefaultAppShellState("Asia/Seoul"),
    discovery: {
      status: "ready",
      chats: [selectedChat]
    },
    selectedChats: [selectedChat],
    providerCredentialStatus: "configured",
    decisionEvidence: decisionEvidenceReport
  }
}

function createBridgeMock() {
  const schedulerState = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  } as const satisfies SyncSchedulerState

  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async (_state: NativeAppShellState) => undefined),
    getRuntimeIdentity: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async (_onState: (state: NativeAppShellState) => void) => vi.fn()),
    subscribeMenuCommand: vi.fn(async (_onCommand: (command: NativeMenuCommand) => void) => vi.fn()),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async (): Promise<SyncScanResult> => {
      const result = {
        pendingProposalCount: 2,
        createdCandidateCount: 1,
        quietLogCount: 0,
        createdExternalProposalCount: 0,
        failedExternalProposalCount: 0,
        feedbackLabelCount: 0,
        featureSnapshotCount: 0,
        latestEvalStatus: "never_run",
        createdCandidateIds: ["candidate-alpha"]
      } as const
      return result
    }),
    discoverMessagesChats: vi.fn(async () => undefined),
    loadMessagesChatPreviews: vi.fn(async () => undefined),
    getPermissionStatuses: vi.fn(async () => undefined),
    storeMorrowToken: vi.fn(async () => undefined),
    readMorrowToken: vi.fn(async () => undefined),
    deleteMorrowToken: vi.fn(async () => undefined),
    checkProviderAuth: vi.fn(async () => undefined),
    getSyncSchedulerState: vi.fn(async () => schedulerState),
    setSyncSchedulerState: vi.fn(async (state: SyncSchedulerState) => state),
    deleteMorrowData: vi.fn(async (_request: MorrowDataDeleteRequest): Promise<MorrowDataDeleteReceipt | undefined> => undefined),
    openPrivacySettings: vi.fn(async (): Promise<PrivacySettingsReceipt | undefined> => undefined),
    recordCrashLog: vi.fn(async () => undefined),
    loadDecisionEvidence: vi.fn(async () => decisionEvidenceReport)
  }
}

type BridgeMock = ReturnType<typeof createBridgeMock>

function Harness({
  bridge,
  initialState
}: {
  readonly bridge: BridgeMock
  readonly initialState: AppShellState
}) {
  const [state, setState] = useState(initialState)
  const [syncing, setSyncing] = useState(false)
  const syncInFlight = useRef(false)
  const actions = useSyncSchedulerOrchestrator({
    nativeBridge: bridge,
    state,
    setState,
    setSyncing,
    syncInFlight
  })

  return (
    <>
      <button type="button" onClick={actions.runSyncNow}>
        Sync Now
      </button>
      <output data-testid="syncing">{syncing ? "syncing" : "idle"}</output>
      <output data-testid="pending-count">{state.pendingProposalCount}</output>
      <output data-testid="decision-evidence-count">{state.decisionEvidence.items.length}</output>
    </>
  )
}

describe("useSyncSchedulerOrchestrator decision evidence", () => {
  beforeEach(() => {
    vi.useRealTimers()
  })

  it("loads decision evidence with created candidate ids after a successful scan", async () => {
    // Given
    const bridge = createBridgeMock()
    render(<Harness bridge={bridge} initialState={readyStateWithDecisionEvidence()} />)

    // When
    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))

    // Then
    await waitFor(() => expect(bridge.loadDecisionEvidence).toHaveBeenCalledOnce())
    expect(bridge.loadDecisionEvidence).toHaveBeenCalledWith({
      createdCandidateIds: ["candidate-alpha"],
      limit: 20
    })
    expect(screen.getByTestId("pending-count")).toHaveTextContent("2")
    expect(screen.getByTestId("decision-evidence-count")).toHaveTextContent("1")
  })

  it("clears stale decision evidence when scan fails before evidence loading", async () => {
    // Given
    const bridge = createBridgeMock()
    bridge.scanSelectedChats.mockRejectedValueOnce(new Error("Provider timed out."))
    render(<Harness bridge={bridge} initialState={readyStateWithDecisionEvidence()} />)

    // When
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))
    })

    // Then
    await waitFor(() => expect(bridge.scanSelectedChats).toHaveBeenCalledOnce())
    expect(bridge.loadDecisionEvidence).not.toHaveBeenCalled()
    expect(screen.getByTestId("decision-evidence-count")).toHaveTextContent("0")
  })
})
