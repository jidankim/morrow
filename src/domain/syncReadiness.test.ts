import { describe, expect, it } from "vitest"
import {
  createDefaultAppShellState,
  isSyncNowEnabled,
  reduceAppShellState,
  type AppShellState
} from "./appShell"
import { getSyncReadinessItems } from "./syncReadiness"

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

describe("sync readiness", () => {
  it("getSyncReadinessItems marks setup complete when permissions, discovery, and selection are ready", () => {
    // Given
    const ready = createReadyAppShellState()

    // When
    const items = getSyncReadinessItems(ready)

    // Then
    expect(items).toEqual([
      {
        id: "permissions",
        label: "Required permissions",
        status: "complete",
        detail: "Required permissions are complete."
      },
      {
        id: "discovery",
        label: "Messages discovery",
        status: "complete",
        detail: "Messages discovery found 1 eligible chat."
      },
      {
        id: "chat-selection",
        label: "Chat selection",
        status: "complete",
        detail: "1 chat selected."
      },
      {
        id: "selected-chat-verification",
        label: "Selected chat verification",
        status: "complete",
        detail: "Selected chats are verified for scanning."
      },
      {
        id: "pause-state",
        label: "Scanning state",
        status: "complete",
        detail: "Scanning is active."
      },
      {
        id: "sync-activity",
        label: "Sync Now activity",
        status: "complete",
        detail: "Sync Now is not already running."
      }
    ])
    expect(isSyncNowEnabled(ready)).toBe(true)
  })

  it("getSyncReadinessItems marks stale selected chats as blocking", () => {
    // Given
    const staleSelected = {
      ...createReadyAppShellState(),
      discovery: {
        status: "ready",
        chats: [{ ...discoveredChat, latestActivityTimestamp: 1_783_000_001 }]
      }
    } satisfies AppShellState

    // When
    const items = getSyncReadinessItems(staleSelected)

    // Then
    expect(items).toContainEqual({
      id: "selected-chat-verification",
      label: "Selected chat verification",
      status: "blocking",
      detail: "Refresh chat discovery before scanning selected chats."
    })
    expect(isSyncNowEnabled(staleSelected)).toBe(false)
  })

  it("getSyncReadinessItems explains every Sync Now disabled setup state", () => {
    // Given
    const ready = createReadyAppShellState()
    const cases = [
      {
        state: { ...ready, config: { ...ready.config, permissionsGranted: false } },
        item: "permissions",
        detail: "Complete required permissions before scanning."
      },
      {
        state: { ...ready, discovery: { status: "unverified", chats: [] } },
        item: "discovery",
        detail: "Run chat discovery before scanning."
      },
      {
        state: { ...ready, discovery: { status: "loading", chats: [] } },
        item: "discovery",
        detail: "Morrow is checking local Messages access."
      },
      {
        state: { ...ready, discovery: { status: "empty", chats: [] } },
        item: "discovery",
        detail: "Messages discovery found no eligible chats."
      },
      {
        state: { ...ready, discovery: { status: "permissionDenied", chats: [] } },
        item: "discovery",
        detail: "Grant Full Disk Access, then retry chat discovery."
      },
      {
        state: { ...ready, discovery: { status: "unavailable", chats: [] } },
        item: "discovery",
        detail: "Retry chat discovery or check local Messages access."
      },
      {
        state: { ...ready, selectedChats: [] },
        item: "chat-selection",
        detail: "Select at least one chat before scanning."
      },
      {
        state: reduceAppShellState(ready, { type: "pause" }),
        item: "pause-state",
        detail: "Resume scanning to enable Sync Now."
      },
      {
        state: ready,
        options: { syncing: true },
        item: "sync-activity",
        detail: "Sync Now is already running."
      }
    ] satisfies readonly {
      readonly state: AppShellState
      readonly options?: { readonly syncing: boolean }
      readonly item: ReturnType<typeof getSyncReadinessItems>[number]["id"]
      readonly detail: string
    }[]

    for (const testCase of cases) {
      // When
      const item = getSyncReadinessItems(testCase.state, testCase.options).find(
        (readinessItem) => readinessItem.id === testCase.item
      )

      // Then
      expect(item).toMatchObject({
        status: "blocking",
        detail: testCase.detail
      })
    }
  })
})

function createReadyAppShellState(): AppShellState {
  const initial = createDefaultAppShellState()
  return {
    ...initial,
    config: { ...initial.config, permissionsGranted: true },
    discovery: { status: "ready", chats: [discoveredChat] },
    selectedChats: [selectedChat]
  }
}
