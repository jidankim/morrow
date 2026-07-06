import { Download, LogIn, RefreshCw } from "lucide-react"

export type ProviderCredentialState =
  | { readonly status: "idle" }
  | { readonly status: "checking" }
  | { readonly status: "missingCli" }
  | { readonly status: "installConfirming" }
  | { readonly status: "installing" }
  | { readonly status: "notLoggedIn" }
  | { readonly status: "loginLaunching" }
  | { readonly status: "loginPolling" }
  | { readonly status: "ready" }
  | {
      readonly status: "failed"
      readonly message: string
      readonly recoveryAction: "refresh" | "setup" | "login"
    }

type SettingsProviderCredentialSectionProps = {
  readonly state: ProviderCredentialState
  readonly onCheckProviderCredential: () => Promise<void>
  readonly onConfirmInstallCodexCli: () => Promise<void>
  readonly onCancelInstallCodexCli: () => void
  readonly onStartCodexLogin: () => Promise<void>
}

export function SettingsProviderCredentialSection({
  state,
  onCheckProviderCredential,
  onConfirmInstallCodexCli,
  onCancelInstallCodexCli,
  onStartCodexLogin
}: SettingsProviderCredentialSectionProps): JSX.Element {
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
        <ProviderCredentialAction
          state={state}
          onCancelInstallCodexCli={onCancelInstallCodexCli}
          onCheckProviderCredential={onCheckProviderCredential}
          onConfirmInstallCodexCli={onConfirmInstallCodexCli}
          onStartCodexLogin={onStartCodexLogin}
        />
      </div>
      <ProviderCredentialMessage state={state} />
    </section>
  )
}

function ProviderCredentialAction({
  state,
  onCheckProviderCredential,
  onConfirmInstallCodexCli,
  onCancelInstallCodexCli,
  onStartCodexLogin
}: SettingsProviderCredentialSectionProps): JSX.Element {
  switch (state.status) {
    case "idle":
    case "ready":
      return (
        <button className="button primary" onClick={() => void onCheckProviderCredential()} type="button">
          <RefreshCw aria-hidden="true" size={16} />
          Refresh readiness
        </button>
      )
    case "checking":
      return (
        <button className="button primary" disabled type="button">
          <RefreshCw aria-hidden="true" size={16} />
          Checking
        </button>
      )
    case "missingCli":
      return (
        <button className="button primary" onClick={() => void onConfirmInstallCodexCli()} type="button">
          <Download aria-hidden="true" size={16} />
          Install Codex CLI
        </button>
      )
    case "installConfirming":
      return (
        <>
          <button className="button primary" onClick={() => void onConfirmInstallCodexCli()} type="button">
            <Download aria-hidden="true" size={16} />
            Install
          </button>
          <button className="button secondary" onClick={onCancelInstallCodexCli} type="button">
            Cancel
          </button>
        </>
      )
    case "installing":
      return (
        <button className="button primary" disabled type="button">
          <Download aria-hidden="true" size={16} />
          Installing Codex CLI
        </button>
      )
    case "notLoggedIn":
      return (
        <button className="button primary" onClick={() => void onStartCodexLogin()} type="button">
          <LogIn aria-hidden="true" size={16} />
          Start Codex login
        </button>
      )
    case "loginLaunching":
      return (
        <button className="button primary" disabled type="button">
          <LogIn aria-hidden="true" size={16} />
          Starting Codex login
        </button>
      )
    case "loginPolling":
      return (
        <button className="button primary" disabled type="button">
          <LogIn aria-hidden="true" size={16} />
          Waiting for browser login
        </button>
      )
    case "failed":
      switch (state.recoveryAction) {
        case "setup":
          return (
            <button className="button primary" onClick={() => void onConfirmInstallCodexCli()} type="button">
              <Download aria-hidden="true" size={16} />
              Retry setup
            </button>
          )
        case "login":
          return (
            <button className="button primary" onClick={() => void onStartCodexLogin()} type="button">
              <LogIn aria-hidden="true" size={16} />
              Start Codex login
            </button>
          )
        case "refresh":
          return (
            <button className="button primary" onClick={() => void onCheckProviderCredential()} type="button">
              <RefreshCw aria-hidden="true" size={16} />
              Refresh readiness
            </button>
          )
        default:
          return assertNever(state.recoveryAction)
      }
    default:
      return assertNever(state)
  }
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
    case "missingCli":
      return <p className="inline-status">Codex CLI is not installed.</p>
    case "installConfirming":
      return <p className="inline-status">Install Codex CLI now?</p>
    case "installing":
      return <p className="inline-status">Installing Codex CLI...</p>
    case "notLoggedIn":
      return <p className="inline-status">Codex CLI is installed. ChatGPT login is required.</p>
    case "loginLaunching":
      return <p className="inline-status">Starting Codex browser login...</p>
    case "loginPolling":
      return <p className="inline-status">Waiting for browser login to complete...</p>
    case "ready":
      return <p className="inline-status success">Codex provider is ready.</p>
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

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential variant: ${String(value)}`)
}
