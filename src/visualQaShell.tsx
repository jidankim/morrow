export function VisualQaShell({ stateName, lede = "Full Disk Access recovery visual QA", children }: {
  readonly stateName: string
  readonly lede?: string | undefined
  readonly children: JSX.Element
}): JSX.Element {
  return (
    <main className="app-shell visual-qa-shell" data-visual-qa-state={stateName}>
      <aside className="sidebar" aria-label="Visual QA fixture">
        <h1>Morrow</h1>
        <p className="lede">{lede}</p>
      </aside>
      <section className="content" aria-live="polite">{children}</section>
    </main>
  )
}
