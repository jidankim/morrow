import { Eye, EyeOff, RefreshCw } from "lucide-react"
import type { ChatId } from "./domain/appShell"

export type ChatPreviewDisclosure = {
  readonly status: "hidden" | "loading" | "ready" | "failed"
  readonly previews: ReadonlyMap<ChatId, string>
}

type ChatPreviewControlsProps = {
  readonly previewDisclosure: ChatPreviewDisclosure
  readonly onRevealPreviews: () => void
  readonly onHidePreviews: () => void
  readonly onRetry: () => void
}

export function ChatPreviewControls({
  previewDisclosure,
  onRevealPreviews,
  onHidePreviews,
  onRetry
}: ChatPreviewControlsProps): JSX.Element {
  const isReady = previewDisclosure.status === "ready"
  const isLoading = previewDisclosure.status === "loading"
  const label = isReady ? "Hide previews" : isLoading ? "Loading previews" : "Reveal previews locally"

  return (
    <div className="chat-preview-toolbar">
      <button
        className="button secondary chat-preview-toggle"
        data-visual-qa-control="chat-preview-toggle"
        disabled={isLoading}
        onClick={isReady ? onHidePreviews : onRevealPreviews}
        type="button"
      >
        {isReady ? <EyeOff aria-hidden="true" size={15} /> : <Eye aria-hidden="true" size={15} />}
        {label}
      </button>
      <button
        className="button secondary chat-preview-toggle"
        data-visual-qa-control="retry-discovery"
        onClick={onRetry}
        type="button"
      >
        <RefreshCw aria-hidden="true" size={15} />
        Retry chat discovery
      </button>
      {previewDisclosure.status === "failed" ? (
        <p className="chat-preview-status" role="status">
          Previews unavailable. Try again.
        </p>
      ) : null}
    </div>
  )
}
