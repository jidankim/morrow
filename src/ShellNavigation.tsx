import { ChartColumnIncreasing, ClipboardList, Settings, SlidersHorizontal } from "lucide-react"
import type { Route } from "./appRuntime"

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
        aria-current={route === "usage" ? "page" : undefined}
        className={route === "usage" ? "nav-link active" : "nav-link"}
        href="#usage"
      >
        <ChartColumnIncreasing aria-hidden="true" size={17} />
        Usage
      </a>
      <a
        aria-current={route === "list-intake" ? "page" : undefined}
        className={route === "list-intake" ? "nav-link active" : "nav-link"}
        href="#list-intake"
      >
        <ClipboardList aria-hidden="true" size={17} />
        List intake
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
