import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import "./styles.css"

if (import.meta.env.DEV && import.meta.env["VITE_DISABLE_REACT_DEVTOOLS"] !== "1") {
  void import("react-grab")
  void import("react-scan")
}

const rootElement = document.getElementById("root")

if (rootElement === null) {
  throw new Error("Morrow root element is missing")
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>
)
