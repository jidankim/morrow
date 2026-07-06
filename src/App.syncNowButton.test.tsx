import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"

describe("App Sync Now button", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("disables Sync Now while manual sync is in flight", async () => {
    // Given
    let finishReconcile = (): void => undefined
    bridgeMock.reconcileNow.mockImplementationOnce(
      async () =>
        await new Promise<void>((resolve) => {
          finishReconcile = resolve
        })
    )
    seedReadyState()
    render(<App />)

    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())

    // When
    fireEvent.click(syncButton)

    // Then
    const runningButton = await screen.findByRole("button", { name: "Syncing" })
    expect(runningButton).toBeDisabled()
    expect(runningButton).toHaveAttribute("aria-busy", "true")
    await waitFor(() =>
      expect(bridgeMock.setShellState).toHaveBeenCalledWith(
        expect.objectContaining({ syncNowRunning: true })
      )
    )
    fireEvent.click(runningButton)
    expect(bridgeMock.reconcileNow).toHaveBeenCalledOnce()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()

    await act(async () => {
      finishReconcile()
    })

    await waitFor(() => expect(screen.getByTestId("pending-count")).toHaveTextContent("9+"))
    const readyButton = screen.getByRole("button", { name: "Sync Now" })
    expect(readyButton).toBeEnabled()
    expect(readyButton).not.toHaveAttribute("aria-busy")
    await waitFor(() =>
      expect(bridgeMock.setShellState).toHaveBeenLastCalledWith(
        expect.objectContaining({ syncNowRunning: false })
      )
    )
  })
})
