import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type { ChatDiscovery, ChatId } from "./domain/appShell"
import type { NativeShellBridge } from "./tauriBridge"

export type MessagesPreviewDisclosureStatus = "hidden" | "loading" | "ready" | "failed"

export type MessagesPreviewDisclosure = {
  readonly status: MessagesPreviewDisclosureStatus
  readonly previews: ReadonlyMap<ChatId, string>
}

type MessagesPreviewDisclosureParams = {
  readonly discovery: ChatDiscovery
  readonly nativeBridge: Pick<NativeShellBridge, "loadMessagesChatPreviews">
}

type MessagesPreviewDisclosureActions = {
  readonly previewDisclosure: MessagesPreviewDisclosure
  readonly revealPreviews: () => void
  readonly hidePreviews: () => void
}

const emptyPreviews: ReadonlyMap<ChatId, string> = new Map<ChatId, string>()

export function useMessagesPreviewDisclosure({
  discovery,
  nativeBridge
}: MessagesPreviewDisclosureParams): MessagesPreviewDisclosureActions {
  const [previewDisclosure, setPreviewDisclosure] = useState<MessagesPreviewDisclosure>({
    status: "hidden",
    previews: emptyPreviews
  })
  const requestVersion = useRef(0)
  const visibleChatIds = useMemo(
    () => (discovery.status === "ready" ? discovery.chats.map((chat) => chat.id) : []),
    [discovery]
  )
  const visibleChatIdKey = visibleChatIds.join("\n")

  useEffect(() => {
    requestVersion.current += 1
    setPreviewDisclosure({ status: "hidden", previews: emptyPreviews })
  }, [visibleChatIdKey])

  const hidePreviews = useCallback((): void => {
    requestVersion.current += 1
    setPreviewDisclosure({ status: "hidden", previews: emptyPreviews })
  }, [])

  const revealPreviews = useCallback((): void => {
    if (discovery.status !== "ready") {
      return
    }
    const chatIds = discovery.chats.map((chat) => chat.id)
    if (chatIds.length === 0) {
      return
    }

    const currentRequestVersion = requestVersion.current + 1
    const visibleChatIdsForRequest = new Set(chatIds)
    requestVersion.current = currentRequestVersion
    setPreviewDisclosure({ status: "loading", previews: emptyPreviews })

    void nativeBridge.loadMessagesChatPreviews({ chatIds }).then(
      (report) => {
        if (requestVersion.current !== currentRequestVersion) {
          return
        }
        const nextPreviews = new Map<ChatId, string>()
        for (const row of report?.chats ?? []) {
          if (visibleChatIdsForRequest.has(row.chatId)) {
            nextPreviews.set(row.chatId, row.preview)
          }
        }
        setPreviewDisclosure({ status: "ready", previews: nextPreviews })
      },
      () => {
        if (requestVersion.current !== currentRequestVersion) {
          return
        }
        setPreviewDisclosure({ status: "failed", previews: emptyPreviews })
      }
    )
  }, [discovery, nativeBridge])

  return { previewDisclosure, revealPreviews, hidePreviews }
}
