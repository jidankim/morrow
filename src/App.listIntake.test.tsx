import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import {
  createBrowserShellStorage,
  loadAppShellState
} from "./domain/appShell"
import { VisualListIntakeHarness } from "./VisualListIntakeHarness"
import { visualListIntakeReport } from "./visualQaListIntakeData"

describe("App list intake settings", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
  })

  it("creates, edits, persists, reloads, and deletes a list-intake profile", async () => {
    seedReadyState()
    const firstRender = render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    typeTextIntoInput("Profile name", "Fish count")
    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: "2 anchovies, 3 salmon\nsalmon - 3 / anchovy - 2" }
    })
    fireEvent.change(screen.getByLabelText("Negative examples"), {
      target: { value: "remind me to buy fish tomorrow" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Enable profile" }))

    await waitFor(() => {
      const reloaded = loadAppShellState(createBrowserShellStorage(window.localStorage))
      expect(reloaded.config.listIntakeProfiles).toEqual([
        expect.objectContaining({
          enabled: true,
          name: "Fish count",
          positiveExamples: ["2 anchovies, 3 salmon", "salmon - 3 / anchovy - 2"],
          negativeExamples: ["remind me to buy fish tomorrow"]
        })
      ])
    })

    firstRender.unmount()
    window.location.hash = "#settings"
    render(<App />)
    expect(await screen.findByDisplayValue("Fish count")).toBeInTheDocument()
    expect(screen.getByText("Rejected negative example: remind me to buy fish tomorrow")).toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Delete profile" }))
    await waitFor(() => {
      const reloaded = loadAppShellState(createBrowserShellStorage(window.localStorage))
      expect(reloaded.config.listIntakeProfiles).toEqual([])
    })
  })
})

describe("App list intake review surface", () => {
  it("loads aggregate groups through the product route without private ids", async () => {
    seedReadyState()
    bridgeMock.setListIntakeReviewReport(visualListIntakeReport)
    render(<App />)

    act(() => {
      window.location.hash = "#list-intake"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(await screen.findByRole("heading", { name: "List intake" })).toBeInTheDocument()
    expect(bridgeMock.loadListIntakeReview).toHaveBeenCalled()
    expect(screen.getByText("Recent captured aggregates")).toBeInTheDocument()
    expect(screen.getByText("Fish count")).toBeInTheDocument()
    expect(screen.getByText("2026-07-07")).toBeInTheDocument()
    expect(screen.getAllByText("Chat alpha").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Sender 1").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Seafood").length).toBeGreaterThan(0)
    expect(screen.getByLabelText("Aggregate item rows")).toHaveTextContent("5 anchovies")
    expect(screen.getByLabelText("Aggregate item rows")).toHaveTextContent("3 salmon")
    expect(document.body).not.toHaveTextContent(/senderKey|message-guid|provider JSON|\+15555550103|Daily list reminders/i)
  })

  it("validates edited approval and sends approve through the product bridge", async () => {
    seedReadyState()
    bridgeMock.setListIntakeReviewReport(visualListIntakeReport)
    render(<App />)

    act(() => {
      window.location.hash = "#list-intake"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(await screen.findByText("List intake proposal")).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText("Item name"), { target: { value: "sardines" } })
    fireEvent.change(screen.getByLabelText("Quantity"), { target: { value: "7" } })
    expect(screen.getByLabelText("Item name")).toHaveDisplayValue("sardines")
    expect(screen.getByLabelText("Quantity")).toHaveDisplayValue("7")
    expect(screen.getByLabelText("Unit")).toHaveDisplayValue("count")

    fireEvent.change(screen.getByLabelText("Quantity"), { target: { value: "1000" } })
    fireEvent.click(screen.getByRole("button", { name: "Approve list-intake proposal" }))
    expect(screen.getByRole("alert")).toHaveTextContent("proposal-edit-error")

    fireEvent.change(screen.getByLabelText("Quantity"), { target: { value: "7" } })
    fireEvent.click(screen.getByRole("button", { name: "Approve list-intake proposal" }))
    await waitFor(() => {
      expect(bridgeMock.decideListIntakeProposal).toHaveBeenCalledWith({
        decision: "approveEdited",
        proposalId: "list-intake-proposal-fixture",
        items: [
          {
            itemName: "sardines",
            quantity: 7,
            unit: "count",
            categoryId: "seafood",
            categoryLabel: "Seafood"
          }
        ]
      })
    })
    expect(screen.getByText("Approved list-intake proposal.")).toBeInTheDocument()
    expect(screen.queryByText("Needs review")).not.toBeInTheDocument()
    expect(screen.getByText("No list-intake proposals need review.")).toBeInTheDocument()
    expect(document.body).not.toHaveTextContent(/senderKey|message-guid|provider JSON|\+15555550103|Daily list reminders/i)
  })

  it("sends reject decisions through the product bridge and visibly clears the pending card", async () => {
    seedReadyState()
    bridgeMock.setListIntakeReviewReport(visualListIntakeReport)
    render(<App />)

    act(() => {
      window.location.hash = "#list-intake"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(await screen.findByText("List intake proposal")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Reject list-intake proposal" }))
    await waitFor(() => {
      expect(bridgeMock.decideListIntakeProposal).toHaveBeenCalledWith({
        decision: "reject",
        proposalId: "list-intake-proposal-fixture"
      })
    })
    expect(screen.getByText("Rejected list-intake proposal.")).toBeInTheDocument()
    expect(screen.queryByText("Needs review")).not.toBeInTheDocument()
    expect(screen.getByText("No list-intake proposals need review.")).toBeInTheDocument()
  })
})

describe("List intake visual review surface", () => {
  it("keeps visual QA aggregate and proposal states rendering product components", () => {
    const aggregateRender = render(<VisualListIntakeHarness stateName="aggregate-populated" />)
    expect(screen.getByText("Recent captured aggregates")).toBeInTheDocument()
    expect(screen.getByText("Fish count")).toBeInTheDocument()
    aggregateRender.unmount()

    render(<VisualListIntakeHarness stateName="proposal-review" />)
    expect(screen.getByText("Proposed items queue")).toBeInTheDocument()
    expect(screen.getByText("List intake proposal")).toBeInTheDocument()
  })
})

function typeTextIntoInput(label: string, text: string): void {
  const field = screen.getByLabelText(label)
  if (!(field instanceof HTMLInputElement)) {
    throw new TypeError(`${label} is not an input`)
  }
  fireEvent.change(field, { target: { value: "" } })
  for (const character of text) {
    fireEvent.change(field, { target: { value: `${field.value}${character}` } })
  }
}
