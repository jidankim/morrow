import { useEffect, useMemo, useState } from "react"
import { SettingsView } from "./SettingsView"
import { ShellNavigation } from "./ShellNavigation"
import { StatusView } from "./StatusView"
import { createNativeShellBridge, type RuntimeIdentity } from "./tauriBridge"
import { useAppShellController } from "./useAppShellController"

export function App(): JSX.Element {
  const shell = useAppShellController()
  const runtimeIdentityBridge = useMemo(() => createNativeShellBridge(), [])
  const [runtimeIdentity, setRuntimeIdentity] = useState<RuntimeIdentity | undefined>(undefined)

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

  return (
    <main className="app-shell">
      <ShellNavigation route={shell.route} />
      <section className="content" aria-live="polite">
        {shell.route === "settings" ? (
          <SettingsView
            config={shell.state.config}
            deleteAllState={shell.deleteAllState}
            providerCredentialState={shell.providerCredentialState}
            runtimeIdentity={runtimeIdentity}
            onChange={shell.updateConfig}
            onCheckProviderCredential={shell.checkProviderCredential}
            onDeleteAll={shell.deleteAllMorrowData}
            onOpenPrivacySettings={shell.openPrivacySettings}
          />
        ) : (
          <StatusView
            menu={shell.menu}
            state={shell.state}
            warnings={shell.warnings}
            syncing={shell.syncing}
            syncEnabled={shell.syncEnabled}
            runtimeIdentity={runtimeIdentity}
            previewDisclosure={shell.previewDisclosure}
            onPause={() => shell.setMode("paused")}
            onResume={() => shell.setMode("scanning")}
            onSyncNow={shell.runSyncNow}
            onHidePreviews={shell.hidePreviews}
            onRetryChatDiscovery={shell.loadMessagesDiscovery}
            onRevealPreviews={shell.revealPreviews}
            onOpenFullDiskAccess={shell.openFullDiskAccess}
            onOpenSettings={shell.openSettingsRoute}
            onToggleChat={shell.toggleChat}
            onToggleBackfillPrompt={shell.toggleBackfillPrompt}
          />
        )}
      </section>
    </main>
  )
}
