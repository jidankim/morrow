import { describe, expect, it } from "vitest"
import { APP_SHELL_STATE_KEY, loadAppShellState } from "./appShell"

describe("app shell privacy boundaries", () => {
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
})
