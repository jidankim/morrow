import { fireEvent, screen, waitFor } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { RuntimeIdentity } from "./tauriBridge"

type NativeBlockedDiscoveryReport =
  | { readonly status: "permissionDenied"; readonly chats: readonly [] }
  | { readonly status: "unavailable"; readonly chats: readonly [] }

type RuntimeIdentityMock = {
  readonly mockResolvedValueOnce: (
    identity: RuntimeIdentity | undefined
  ) => RuntimeIdentityMock
  readonly mockRejectedValueOnce: (error: unknown) => RuntimeIdentityMock
}

type DiscoveryMock = {
  readonly mockResolvedValueOnce: (
    report: NativeBlockedDiscoveryReport
  ) => DiscoveryMock
}

type DiscoveryGuideBridgeMock = {
  readonly getRuntimeIdentity: RuntimeIdentityMock
  readonly discoverMessagesChats: DiscoveryMock
  readonly openPrivacySettings: unknown
}

type DiscoveryGuideTestContext = {
  readonly renderApp: () => void
  readonly bridgeMock: DiscoveryGuideBridgeMock
}

const binaryRuntimeIdentityFixture = {
  displayName: "Morrow",
  bundleIdentifier: "dev.morrow.desktop",
  executablePath: "/Users/example/morrow/target/debug/morrow",
  settingsTargetPath: "/Users/example/morrow/target/debug/morrow",
  runtimeKind: "binary"
} as const satisfies RuntimeIdentity

const appBundleRuntimeIdentityFixture = {
  displayName: "Morrow",
  bundleIdentifier: "dev.morrow.desktop",
  executablePath: "/Applications/Morrow.app/Contents/MacOS/morrow",
  settingsTargetPath: "/Applications/Morrow.app",
  runtimeKind: "appBundle"
} as const satisfies RuntimeIdentity

const deniedReport = { status: "permissionDenied", chats: [] } as const
const unavailableReport = { status: "unavailable", chats: [] } as const

export function registerDiscoveryGuideTests({
  renderApp,
  bridgeMock
}: DiscoveryGuideTestContext): void {
  describe("guided Full Disk Access recovery", () => {
    it("shows guided Full Disk Access recovery for a binary runtime", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(deniedReport)
      stubClipboardSuccess()

      // When
      renderApp()

      // Then
      expect(await screen.findByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
      expect(screen.getByText(/Cmd\+Shift\+G/)).toBeInTheDocument()
      expect(screen.getByText(binaryRuntimeIdentityFixture.settingsTargetPath)).toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Copy Morrow path" })).toBeInTheDocument()
      expect(screen.getAllByText(/restart Morrow, then retry chat discovery/i).length).toBeGreaterThan(0)
      expect(screen.getByText(/Terminal\/Codex Full Disk Access does not grant Morrow access/i)).toBeInTheDocument()

      fireEvent.click(screen.getByRole("button", { name: "Open Full Disk Access" }))

      await waitFor(() =>
        expect(bridgeMock.openPrivacySettings).toHaveBeenCalledWith({ pane: "fullDiskAccess" })
      )
    })

    it("shows guided Full Disk Access recovery for a bundled runtime", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(appBundleRuntimeIdentityFixture)
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(deniedReport)

      // When
      renderApp()

      // Then
      expect(await screen.findByText(/enable or add Morrow\.app/i)).toBeInTheDocument()
      expect(screen.getByText(appBundleRuntimeIdentityFixture.settingsTargetPath)).toBeInTheDocument()
      expect(document.body).not.toHaveTextContent("target/debug")
    })

    it("shows guided Full Disk Access recovery with retry when discovery is unavailable", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(unavailableReport)

      // When
      renderApp()

      // Then
      expect(await screen.findByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Retry chat discovery" })).toBeInTheDocument()
      expect(screen.getAllByText(/restart Morrow, then retry chat discovery/i).length).toBeGreaterThan(0)
    })

    it("does not show a copy path when runtime identity is unavailable", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockRejectedValueOnce(new Error("identity unavailable"))
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(deniedReport)

      // When
      renderApp()

      // Then
      expect(await screen.findByText(/app that launched Morrow/i)).toBeInTheDocument()
      expect(screen.queryByRole("button", { name: "Copy Morrow path" })).not.toBeInTheDocument()
    })

    it("renders copied status after the runtime path is copied", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(deniedReport)
      const writeText = stubClipboardSuccess()

      // When
      renderApp()
      fireEvent.click(await screen.findByRole("button", { name: "Copy Morrow path" }))

      // Then
      await waitFor(() =>
        expect(writeText).toHaveBeenCalledWith(binaryRuntimeIdentityFixture.settingsTargetPath)
      )
      expect(await screen.findByText("Morrow path copied.")).toBeInTheDocument()
    })

    it("renders failed status when the runtime path cannot be copied", async () => {
      // Given
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      bridgeMock.discoverMessagesChats.mockResolvedValueOnce(deniedReport)
      stubClipboardFailure()

      // When
      renderApp()
      fireEvent.click(await screen.findByRole("button", { name: "Copy Morrow path" }))

      // Then
      expect(await screen.findByText("Morrow path could not be copied.")).toBeInTheDocument()
    })
  })
}

function stubClipboardSuccess(): ReturnType<typeof vi.fn> {
  const writeText = vi.fn(async () => undefined)
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText }
  })
  return writeText
}

function stubClipboardFailure(): void {
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: vi.fn(async () => {
        throw new DOMException("denied", "NotAllowedError")
      })
    }
  })
}
