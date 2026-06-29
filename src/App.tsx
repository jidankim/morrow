import { SettingsView } from "./SettingsView"
import { ShellNavigation } from "./ShellNavigation"
import { StatusView } from "./StatusView"
import { useAppShellController } from "./useAppShellController"

export function App(): JSX.Element {
  const shell = useAppShellController()

  return (
    <main className="app-shell">
      <ShellNavigation route={shell.route} />
      <section className="content" aria-live="polite">
        {shell.route === "settings" ? (
          <SettingsView
            config={shell.state.config}
            deleteAllState={shell.deleteAllState}
            providerCredentialState={shell.providerCredentialState}
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
            onPause={() => shell.setMode("paused")}
            onResume={() => shell.setMode("scanning")}
            onSyncNow={shell.runSyncNow}
            onRetryChatDiscovery={shell.loadMessagesDiscovery}
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
