import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import {
  appBundleRuntimeIdentityFixture,
  binaryRuntimeIdentityFixture,
  bridgeMock,
  openAppRoute
} from "./AppPrivacyTestHarness"

export function registerSettingsFullDiskAccessGuideTests(): void {
  describe("settings Full Disk Access guide", () => {
    beforeEach(() => {
      bridgeMock.getRuntimeIdentity.mockClear()
      bridgeMock.getRuntimeIdentity.mockResolvedValue(undefined)
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: {
          writeText: vi.fn(async () => undefined)
        }
      })
    })

    it("shows runtime-aware Full Disk Access guidance in settings", async () => {
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      render(<App />)

      openAppRoute("#settings")

      expect(await screen.findByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
      expect(
        screen.getByText(/Morrow's runtime needs Full Disk Access to read local Messages/)
      ).toBeInTheDocument()
      expect(screen.getByText(binaryRuntimeIdentityFixture.settingsTargetPath)).toBeInTheDocument()
      expect(screen.getByText(/Cmd\+Shift\+G/)).toBeInTheDocument()
      expect(screen.getByText(/Terminal\/Codex access is only for terminal QA/)).toBeInTheDocument()

      fireEvent.click(screen.getByRole("button", { name: "Copy Morrow path" }))
      await waitFor(() =>
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
          binaryRuntimeIdentityFixture.settingsTargetPath
        )
      )
      fireEvent.click(screen.getByRole("button", { name: "Open Calendar access" }))
      fireEvent.click(screen.getByRole("button", { name: "Open Reminders access" }))
      fireEvent.click(screen.getByRole("button", { name: "Open Full Disk Access" }))

      await waitFor(() => expect(bridgeMock.openPrivacySettings).toHaveBeenCalledTimes(3))
      expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(1, { pane: "calendar" })
      expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(2, { pane: "reminders" })
      expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(3, { pane: "fullDiskAccess" })
    })

    it("shows bundled app Full Disk Access guidance without a dev binary path", async () => {
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(appBundleRuntimeIdentityFixture)
      render(<App />)

      openAppRoute("#settings")

      expect(await screen.findAllByText(/Morrow\.app/)).toHaveLength(2)
      expect(screen.getByText(appBundleRuntimeIdentityFixture.settingsTargetPath)).toBeInTheDocument()
      expect(document.body).not.toHaveTextContent("target/debug")
    })

    it("keeps Calendar and Reminders privacy panes separate from Full Disk Access", async () => {
      render(<App />)

      openAppRoute("#settings")

      fireEvent.click(screen.getByRole("button", { name: "Open Calendar access" }))
      fireEvent.click(screen.getByRole("button", { name: "Open Reminders access" }))

      await waitFor(() => expect(bridgeMock.openPrivacySettings).toHaveBeenCalledTimes(2))
      expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(1, { pane: "calendar" })
      expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(2, { pane: "reminders" })
    })

    it("reports when the Morrow path cannot be copied", async () => {
      const writeText = vi.fn(async () => {
        throw new Error("clipboard blocked")
      })
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: { writeText }
      })
      bridgeMock.getRuntimeIdentity.mockResolvedValueOnce(binaryRuntimeIdentityFixture)
      render(<App />)

      openAppRoute("#settings")
      fireEvent.click(await screen.findByRole("button", { name: "Copy Morrow path" }))

      expect(await screen.findByRole("alert")).toHaveTextContent("Morrow path could not be copied")
    })

    it("shows generic Full Disk Access guidance without a copy path when runtime identity is unavailable", async () => {
      render(<App />)

      openAppRoute("#settings")

      expect(await screen.findByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
      expect(
        screen.getByText(/Open Full Disk Access, add or enable Morrow, restart Morrow, then retry/)
      ).toBeInTheDocument()
      expect(screen.queryByRole("button", { name: "Copy Morrow path" })).not.toBeInTheDocument()
    })
  })
}
