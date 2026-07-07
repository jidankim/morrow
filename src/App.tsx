import { useEffect, useMemo, useState } from "react"
import { assertNeverRoute } from "./appRuntime"
import { ProviderUsageDashboard } from "./ProviderUsageDashboard"
import { ListIntakeReviewView } from "./ListIntakeReviewView"
import { SettingsView } from "./SettingsView"
import { ShellNavigation } from "./ShellNavigation"
import { StatusView } from "./StatusView"
import { createNativeShellBridge, type RuntimeIdentity } from "./tauriBridge"
import { useAppShellController } from "./useAppShellController"
import { useListIntakeReviewController } from "./useListIntakeReviewController"

export function App(): JSX.Element {
  const shell = useAppShellController()
  const runtimeIdentityBridge = useMemo(() => createNativeShellBridge(), [])
  const [runtimeIdentity, setRuntimeIdentity] = useState<RuntimeIdentity | undefined>(undefined)
  const listIntakeReview = useListIntakeReviewController(shell.route, runtimeIdentityBridge)

  useEffect(() => {
    let active = true

    void runtimeIdentityBridge
      .getRuntimeIdentity()
      .then((identity) => {
        if (active) {
          setRuntimeIdentity(identity)
        }
      })
      .catch((error: unknown) => {
        if (!active) {
          return
        }
        if (!(error instanceof Error) && typeof error !== "string") {
          throw error
        }
        setRuntimeIdentity(undefined)
      })

    return () => {
      active = false
    }
  }, [runtimeIdentityBridge])

  let content: JSX.Element
  switch (shell.route) {
    case "settings":
      content = (
        <SettingsView
          config={shell.state.config}
          deleteAllState={shell.deleteAllState}
          providerCredentialState={shell.providerCredentialState}
          runtimeIdentity={runtimeIdentity}
          syncScheduler={shell.syncScheduler}
          syncSchedulerNowUnixSeconds={shell.syncSchedulerNowUnixSeconds}
          onChangeAutomaticSyncInterval={shell.changeAutomaticSyncInterval}
          onChange={shell.updateConfig}
          onCancelInstallCodexCli={shell.cancelInstallCodexCli}
          onCheckProviderCredential={shell.checkProviderCredential}
          onConfirmInstallCodexCli={shell.confirmInstallCodexCli}
          onDeleteAll={shell.deleteAllMorrowData}
          onOpenPrivacySettings={shell.openPrivacySettings}
          onStartCodexLogin={shell.startCodexLogin}
          onToggleAutomaticSync={shell.toggleAutomaticSync}
        />
      )
      break
    case "usage":
      content = (
        <ProviderUsageDashboard
          selectedWindowKey={shell.providerUsage.selectedWindowKey}
          state={shell.providerUsage.state}
          onWindowChange={shell.providerUsage.changeWindow}
        />
      )
      break
    case "list-intake":
      content = (
        <ListIntakeReviewView
          state={listIntakeReview.state}
          onApproveEdited={listIntakeReview.approveEdited}
          onReject={listIntakeReview.reject}
          onReload={listIntakeReview.reload}
        />
      )
      break
    case "status":
      content = (
        <StatusView
          menu={shell.menu}
          state={shell.state}
          warnings={shell.warnings}
          syncing={shell.syncing}
          syncEnabled={shell.syncEnabled}
          runtimeIdentity={runtimeIdentity}
          previewDisclosure={shell.previewDisclosure}
          syncScheduler={shell.syncScheduler}
          syncSchedulerNowUnixSeconds={shell.syncSchedulerNowUnixSeconds}
          onChangeAutomaticSyncInterval={shell.changeAutomaticSyncInterval}
          onPause={() => shell.setMode("paused")}
          onResume={() => shell.setMode("scanning")}
          onSyncNow={shell.runSyncNow}
          onToggleAutomaticSync={shell.toggleAutomaticSync}
          onHidePreviews={shell.hidePreviews}
          onRetryChatDiscovery={shell.loadMessagesDiscovery}
          onRevealPreviews={shell.revealPreviews}
          onOpenFullDiskAccess={shell.openFullDiskAccess}
          onOpenSettings={shell.openSettingsRoute}
          onToggleChat={shell.toggleChat}
          onToggleBackfillPrompt={shell.toggleBackfillPrompt}
        />
      )
      break
    default:
      assertNeverRoute(shell.route)
  }

  return (
    <main className="app-shell">
      <ShellNavigation route={shell.route} />
      <section className="content" aria-live="polite">
        {content}
      </section>
    </main>
  )
}
