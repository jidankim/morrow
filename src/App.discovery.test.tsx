import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { useState } from "react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  discoveredChat,
  nativeChatFromFixture,
  privatePreviewText,
  rediscoveredChat,
  renderDiscoveryApp,
  resetDiscoveryAppTestState
} from "./App.discoveryHarness"
import { APP_SHELL_STATE_KEY, type ChatDiscovery } from "./domain/appShell"
import { useMessagesPreviewDisclosure } from "./useMessagesPreviewDisclosure"

describe("App Messages chat discovery selection and sync", () => {
  beforeEach(() => {
    resetDiscoveryAppTestState()
  })

  it("loads local previews only after reveal is clicked", async () => {
    const secondaryChat = {
      id: "messages-chat-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      label: "Messages chat",
      participantCount: 1,
      participantIds: ["messages-participant-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"],
      latestActivityTimestamp: 1_783_000_100
    } as const
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({
      status: "ready",
      chats: [nativeChatFromFixture(discoveredChat), nativeChatFromFixture(secondaryChat)]
    })
    bridgeMock.loadMessagesChatPreviews.mockResolvedValueOnce({
      chats: [
        { chatId: discoveredChat.id, preview: "private clinic visit" },
        { chatId: secondaryChat.id, preview: "team sync moved to 3" },
        {
          chatId: "messages-chat-ffffffffffffffffffffffffffffffff",
          preview: "stale private preview"
        }
      ]
    })
    renderDiscoveryApp()

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(screen.getByRole("checkbox", { name: /Messages chat/ })).toBeInTheDocument()
    expect(bridgeMock.loadMessagesChatPreviews).not.toHaveBeenCalled()
    expect(screen.queryByText("private clinic visit")).not.toBeInTheDocument()
    expect(screen.getByRole("checkbox", { name: /Chat alpha/ })).not.toHaveAccessibleName(
      new RegExp(privatePreviewText)
    )

    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    expect(screen.getByRole("button", { name: "Loading previews" })).toBeDisabled()
    expect(await screen.findByText("private clinic visit")).toHaveAttribute(
      "data-visual-qa-text",
      "chat-row-preview"
    )
    expect(screen.getByText("team sync moved to 3")).toBeInTheDocument()
    expect(screen.getByRole("checkbox", { name: new RegExp(privatePreviewText) })).toBeInTheDocument()
    expect(screen.queryByText("stale private preview")).not.toBeInTheDocument()
    expect(bridgeMock.loadMessagesChatPreviews).toHaveBeenCalledWith({
      chatIds: [discoveredChat.id, secondaryChat.id]
    })
    expect(window.localStorage.getItem(APP_SHELL_STATE_KEY)).not.toContain("private clinic visit")

    fireEvent.click(screen.getByRole("checkbox", { name: /Chat alpha/ }))

    await waitFor(() =>
      expect(window.localStorage.getItem(APP_SHELL_STATE_KEY)).not.toContain("private clinic visit")
    )
    fireEvent.click(screen.getByRole("button", { name: "Hide previews" }))
    expect(screen.queryByText("private clinic visit")).not.toBeInTheDocument()
    expect(screen.queryByText("team sync moved to 3")).not.toBeInTheDocument()
    expect(screen.getByRole("checkbox", { name: /Chat alpha/ })).not.toHaveAccessibleName(
      new RegExp(privatePreviewText)
    )
  })

  it("reveals latest local previews in chat rows after explicit opt-in", async () => {
    bridgeMock.loadMessagesChatPreviews.mockResolvedValueOnce({
      chats: [{ chatId: discoveredChat.id, preview: privatePreviewText }]
    })
    renderDiscoveryApp()

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(bridgeMock.loadMessagesChatPreviews).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    expect(await screen.findByText(privatePreviewText)).toHaveAttribute("data-visual-qa-text", "chat-row-preview")
    expect(screen.getByRole("checkbox", { name: new RegExp(privatePreviewText) })).toBeInTheDocument()
  })

  it("keeps empty local previews hidden without failing reveal", async () => {
    bridgeMock.loadMessagesChatPreviews.mockResolvedValueOnce({
      chats: [{ chatId: discoveredChat.id, preview: "" }]
    })
    renderDiscoveryApp()

    const chatCheckbox = await screen.findByRole("checkbox", { name: /Chat alpha/ })
    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    await waitFor(() => expect(screen.getByRole("button", { name: "Hide previews" })).toBeEnabled())
    expect(screen.queryByText("Previews unavailable. Try again.")).not.toBeInTheDocument()
    expect(screen.queryByText("No preview available")).not.toBeInTheDocument()
    expect(document.querySelector("[data-visual-qa-text='chat-row-preview']")).not.toBeInTheDocument()
    expect(chatCheckbox).not.toHaveAccessibleName(/Latest preview:/)
  })

  it("hides and clears previews when chat discovery is retried", async () => {
    function PreviewRetryHarness(): JSX.Element {
      const [discovery, setDiscovery] = useState<ChatDiscovery>({
        status: "ready",
        chats: [discoveredChat]
      })
      const { previewDisclosure, revealPreviews } = useMessagesPreviewDisclosure({
        discovery,
        nativeBridge: bridgeMock
      })
      return (
        <div>
          <button onClick={revealPreviews} type="button">Reveal previews locally</button>
          <button onClick={() => setDiscovery({ status: "loading", chats: [] })} type="button">
            Retry chat discovery
          </button>
          <button onClick={() => setDiscovery({ status: "ready", chats: [rediscoveredChat] })} type="button">
            Finish discovery
          </button>
          {[...previewDisclosure.previews.values()].map((preview) => (
            <p key={preview}>{preview}</p>
          ))}
        </div>
      )
    }
    render(<PreviewRetryHarness />)

    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))
    expect(await screen.findByText("private clinic visit")).toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(screen.queryByText("private clinic visit")).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Finish discovery" }))
    expect(screen.queryByText("private clinic visit")).not.toBeInTheDocument()
    expect(bridgeMock.loadMessagesChatPreviews).toHaveBeenCalledOnce()
  })

  it("keeps chat selection usable when preview loading fails", async () => {
    bridgeMock.loadMessagesChatPreviews
      .mockRejectedValueOnce(new Error("preview load failed"))
      .mockResolvedValueOnce({
        chats: [{ chatId: discoveredChat.id, preview: "private clinic visit" }]
      })
    renderDiscoveryApp()

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    expect(await screen.findByText("Previews unavailable. Try again.")).toBeInTheDocument()
    expect(screen.queryByText("private clinic visit")).not.toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Reveal previews locally" })).toBeEnabled()

    fireEvent.click(screen.getByRole("checkbox", { name: /Chat alpha/ }))

    expect(screen.getByRole("checkbox", { name: /Chat alpha/ })).toBeChecked()
    expect(screen.getByRole("checkbox", { name: "Ask before backfilling older messages" })).toBeChecked()
    expect(screen.getByText("Morrow asks before using older messages from this chat.")).toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    expect(await screen.findByText("private clinic visit")).toBeInTheDocument()
    expect(bridgeMock.loadMessagesChatPreviews).toHaveBeenCalledTimes(2)
  })
})
