import { Settings, SlidersHorizontal } from "lucide-react"

type Route = "status" | "settings"

type ShellNavigationProps = {
  readonly route: Route
}

export function ShellNavigation({ route }: ShellNavigationProps): JSX.Element {
  return (
    <nav className="sidebar" aria-label="Morrow">
      <div>
        <p className="eyebrow">Morrow</p>
        <h1>Local scheduling shell</h1>
      </div>
      <a
        aria-current={route === "status" ? "page" : undefined}
        className={route === "status" ? "nav-link active" : "nav-link"}
        href="#status"
      >
        <SlidersHorizontal aria-hidden="true" size={17} />
        Status
      </a>
      <a
        aria-current={route === "settings" ? "page" : undefined}
        className={route === "settings" ? "nav-link active" : "nav-link"}
        href="#settings"
      >
        <Settings aria-hidden="true" size={17} />
        Settings
      </a>
    </nav>
  )
}
