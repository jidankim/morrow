import { Clipboard, RefreshCw } from "lucide-react"
import { useState } from "react"
import type { CodexAuthStatus } from "./tauriBridge"

export type ProviderCredentialState =
  | { readonly status: "idle" }
  | { readonly status: "checking" }
  | { readonly status: "ready" }
  | { readonly status: "missing"; readonly reason: CodexAuthStatus }
  | { readonly status: "failed"; readonly message: string }

type SettingsProviderCredentialSectionProps = {
  readonly state: ProviderCredentialState
  readonly onCheckProviderCredential: () => Promise<void>
}

const CODEX_LOGIN_COMMAND = "codex login"

export function SettingsProviderCredentialSection({
  state,
  onCheckProviderCredential
}: SettingsProviderCredentialSectionProps): JSX.Element {
  const [copyStatus, setCopyStatus] = useState<"idle" | "copied" | "failed">("idle")
  const actionInFlight = state.status === "checking"

  const copyLoginCommand = async (): Promise<void> => {
    try {
      await navigator.clipboard.writeText(CODEX_LOGIN_COMMAND)
      setCopyStatus("copied")
    } catch (error: unknown) {
      if (isClipboardWriteError(error)) {
        setCopyStatus("failed")
        return
      }
      throw error
    }
  }

  return (
    <section className="settings-section" aria-labelledby="provider-credential-heading">
      <div>
        <p className="eyebrow">Provider readiness</p>
        <h3 id="provider-credential-heading">Codex provider</h3>
      </div>
      <p className="settings-copy">
        Morrow uses your local Codex CLI ChatGPT login for scheduling extraction. No provider
        tokens are stored in Morrow.
      </p>
      <div className="credential-action-row">
        <button
          className="button primary"
          disabled={actionInFlight}
          onClick={() => void onCheckProviderCredential()}
          type="button"
        >
          <RefreshCw aria-hidden="true" size={16} />
          {state.status === "checking" ? "Checking" : "Refresh readiness"}
        </button>
        <button
          className="button secondary"
          onClick={() => void copyLoginCommand()}
          type="button"
        >
          <Clipboard aria-hidden="true" size={16} />
          Copy login command
        </button>
      </div>
      <ProviderCredentialMessage state={state} />
      <ProviderCommandCopyMessage status={copyStatus} />
    </section>
  )
}

function ProviderCredentialMessage({
  state
}: {
  readonly state: ProviderCredentialState
}): JSX.Element | null {
  switch (state.status) {
    case "idle":
      return <p className="inline-status">Codex provider readiness has not been checked.</p>
    case "checking":
      return <p className="inline-status">Checking Codex provider readiness...</p>
    case "ready":
      return <p className="inline-status success">Codex provider is ready.</p>
    case "missing":
      return (
        <p className="inline-status">
          Codex provider is not ready. {setupInstruction(state.reason)}
        </p>
      )
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          {state.message}
        </p>
      )
    default:
      return assertNever(state)
  }
}

function ProviderCommandCopyMessage({
  status
}: {
  readonly status: "idle" | "copied" | "failed"
}): JSX.Element | null {
  switch (status) {
    case "idle":
      return null
    case "copied":
      return <p className="inline-status success">Login command copied.</p>
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          Login command could not be copied.
        </p>
      )
    default:
      return assertNever(status)
  }
}

function setupInstruction(reason: CodexAuthStatus): string {
  switch (reason) {
    case "missingCli":
      return "Install Codex CLI, then run codex login."
    case "notLoggedIn":
      return "Run codex login in Terminal."
    case "timeout":
      return "Refresh readiness or run codex login in Terminal."
    case "unknownFailure":
      return "Refresh readiness or check Codex CLI in Terminal."
    case "loggedInUsingChatGpt":
      return "Refresh readiness."
    default:
      return assertNever(reason)
  }
}

function isClipboardWriteError(error: unknown): error is DOMException | Error {
  return error instanceof DOMException || error instanceof Error
}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential variant: ${String(value)}`)
}
