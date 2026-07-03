import { afterEach, describe, expect, it, vi } from "vitest"
import { chatDisplayMetadata } from "./chatDisplay"

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Team planning",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_783_000_000
} as const

describe("chatDisplayMetadata", () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it("formats latest activity with a resolved concrete timezone for system preference", () => {
    // Given
    const dateTimeFormatSpy = vi.spyOn(Intl, "DateTimeFormat")

    // When
    const metadata = chatDisplayMetadata(discoveredChat, "system", "Asia/Tokyo")

    // Then
    expect(metadata.latestActivityText).toContain("Last active")
    const formattedTimeZones = dateTimeFormatSpy.mock.calls.map((call) => call[1]?.timeZone)
    expect(formattedTimeZones).toContain("Asia/Tokyo")
    expect(formattedTimeZones).not.toContain("system")
  })
})
