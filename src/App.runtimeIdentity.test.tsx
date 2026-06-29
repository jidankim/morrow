import { render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  appBundleRuntimeIdentityFixture,
  binaryRuntimeIdentityFixture,
  bridgeMock,
  openAppRoute
} from "./AppPrivacyTestHarness"
import { App } from "./App"

const runtimeIdentityCases = [
  ["binary", binaryRuntimeIdentityFixture],
  ["appBundle", appBundleRuntimeIdentityFixture]
] as const

describe("App runtime identity propagation", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.getRuntimeIdentity.mockClear()
    bridgeMock.getRuntimeIdentity.mockResolvedValue(undefined)
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.openPrivacySettings.mockClear()
  })

  it("keeps rendering when runtime identity is unavailable", async () => {
    // Given
    bridgeMock.getRuntimeIdentity.mockRejectedValueOnce(new Error("runtime identity unavailable"))

    // When
    render(<App />)

    // Then
    expect(await screen.findByText("App shell")).toBeInTheDocument()
    await waitFor(() => expect(bridgeMock.getRuntimeIdentity).toHaveBeenCalledOnce())
    expect(await screen.findByText("Messages discovery found 2 eligible chats.")).toBeInTheDocument()
  })

  it.each(runtimeIdentityCases)("shows %s runtime identity in the discovery recovery guide", async (_runtimeKind, runtimeIdentity) => {
    // Given
    bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(runtimeIdentity)
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })

    // When
    render(<App />)

    // Then
    expect(await screen.findByText(runtimeIdentity.settingsTargetPath)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Copy Morrow path" })).toBeInTheDocument()
    expect(bridgeMock.getRuntimeIdentity).toHaveBeenCalledOnce()
  })

  it.each(runtimeIdentityCases)(
    "shows %s runtime identity in settings recovery guide",
    async (_runtimeKind, runtimeIdentity) => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(runtimeIdentity)

      // When
      render(<App />)
      openAppRoute("#settings")

      // Then
      expect(await screen.findByText(runtimeIdentity.settingsTargetPath)).toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Copy Morrow path" })).toBeInTheDocument()
      expect(bridgeMock.getRuntimeIdentity).toHaveBeenCalledOnce()
      expect(window.localStorage.getItem("runtimeIdentity")).toBeNull()
      expect(window.localStorage.getItem("runtimeIdentityLoadError")).toBeNull()
    }
  )
})
