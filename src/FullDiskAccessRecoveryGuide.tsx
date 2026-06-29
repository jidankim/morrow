import { Clipboard, DatabaseZap, RefreshCw } from "lucide-react"
import { useState } from "react"
import type { RuntimeIdentity } from "./tauriBridge"

export type FullDiskAccessRecoverySurface = "discovery" | "settings"

export type FullDiskAccessRecoveryGuideProps = {
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly surface: FullDiskAccessRecoverySurface
  readonly onOpenFullDiskAccess: () => void | Promise<void>
  readonly onRetry?: (() => void) | undefined
}

type CopyStatus = "idle" | "copied" | "failed"

export function FullDiskAccessRecoveryGuide({
  runtimeIdentity,
  surface,
  onOpenFullDiskAccess,
  onRetry
}: FullDiskAccessRecoveryGuideProps): JSX.Element {
  const [copyStatus, setCopyStatus] = useState<CopyStatus>("idle")
  const targetPath = runtimeIdentity?.settingsTargetPath

  const copyTargetPath = async (): Promise<void> => {
    if (targetPath === undefined) {
      return
    }
    try {
      await navigator.clipboard.writeText(targetPath)
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
    <div className="full-disk-access-guide" data-recovery-surface={surface}>
      <div className="recovery-heading">
        <DatabaseZap aria-hidden="true" size={15} />
        <div>
          <strong>Full Disk Access recovery</strong>
          <p>
            Morrow can open macOS Full Disk Access settings, but macOS still requires
            you to manually enable or add Morrow.
          </p>
        </div>
      </div>
      <RuntimeTargetGuidance runtimeIdentity={runtimeIdentity} />
      <p className="recovery-copy">
        After changing Full Disk Access, restart Morrow, then retry chat discovery.
      </p>
      <div className="discovery-actions">
        <button
          className="button secondary"
          data-visual-qa-control="open-full-disk-access"
          onClick={() => void onOpenFullDiskAccess()}
          type="button"
        >
          <DatabaseZap aria-hidden="true" size={15} />
          Open Full Disk Access
        </button>
        {targetPath !== undefined ? (
          <button
            className="button secondary"
            onClick={() => void copyTargetPath()}
            type="button"
          >
            <Clipboard aria-hidden="true" size={15} />
            Copy Morrow path
          </button>
        ) : null}
        {onRetry !== undefined ? (
          <button
            className="button secondary"
            data-visual-qa-control="retry-discovery"
            onClick={onRetry}
            type="button"
          >
            <RefreshCw aria-hidden="true" size={15} />
            Retry chat discovery
          </button>
        ) : null}
      </div>
      <CopyStatusMessage status={copyStatus} />
    </div>
  )
}

function RuntimeTargetGuidance({
  runtimeIdentity
}: {
  readonly runtimeIdentity?: RuntimeIdentity | undefined
}): JSX.Element {
  if (runtimeIdentity === undefined) {
    return (
      <p className="recovery-copy">
        Enable or add the app that launched Morrow in Full Disk Access.
      </p>
    )
  }

  switch (runtimeIdentity.runtimeKind) {
    case "binary":
      return (
        <>
          <p className="recovery-copy">
            Add this Morrow executable in the file picker. Press Cmd+Shift+G, paste
            the path, then choose it.
          </p>
          <code className="recovery-path">{runtimeIdentity.settingsTargetPath}</code>
          <p className="recovery-copy">
            Terminal/Codex Full Disk Access does not grant Morrow access.
          </p>
        </>
      )
    case "appBundle":
      return (
        <>
          <p className="recovery-copy">
            Enable or add Morrow.app in Full Disk Access.
          </p>
          <code className="recovery-path">{runtimeIdentity.settingsTargetPath}</code>
        </>
      )
    default:
      return assertNever(runtimeIdentity)
  }
}

function CopyStatusMessage({
  status
}: {
  readonly status: CopyStatus
}): JSX.Element | null {
  switch (status) {
    case "idle":
      return null
    case "copied":
      return <p className="inline-status success">Morrow path copied.</p>
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          Morrow path could not be copied.
        </p>
      )
    default:
      return assertNever(status)
  }
}

function isClipboardWriteError(error: unknown): error is DOMException | Error {
  return error instanceof DOMException || error instanceof Error
}

function assertNever(value: never): never {
  throw new Error(`Unhandled Full Disk Access recovery variant: ${String(value)}`)
}
