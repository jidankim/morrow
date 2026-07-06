import { useCallback, useEffect, useState } from "react"
import { DEFAULT_PROVIDER_USAGE_REQUEST, type Route } from "./appRuntime"
import type { ProviderUsageReport, ProviderUsageWindowKey } from "./domain/providerUsage"
import type { NativeShellBridge } from "./tauriBridge"

export type ProviderUsageState =
  | { readonly status: "idle" }
  | { readonly status: "loading" }
  | { readonly status: "empty" }
  | { readonly status: "loaded"; readonly report: ProviderUsageReport }
  | { readonly status: "failed"; readonly message: string }

export type ProviderUsageController = {
  readonly state: ProviderUsageState
  readonly selectedWindowKey: ProviderUsageWindowKey
  readonly changeWindow: (windowKey: ProviderUsageWindowKey) => void
}

export function useProviderUsageController(
  route: Route,
  nativeBridge: NativeShellBridge
): ProviderUsageController {
  const [providerUsage, setProviderUsage] = useState<ProviderUsageState>({ status: "idle" })
  const [selectedWindowKey, setSelectedWindowKey] = useState<ProviderUsageWindowKey>(
    DEFAULT_PROVIDER_USAGE_REQUEST.windowKey
  )

  const changeWindow = useCallback((windowKey: ProviderUsageWindowKey): void => {
    setSelectedWindowKey(windowKey)
  }, [])

  useEffect(() => {
    if (route !== "usage") {
      return
    }

    let active = true
    setProviderUsage({ status: "loading" })

    void nativeBridge
      .loadProviderUsage({ windowKey: selectedWindowKey })
      .then((report) => {
        if (!active) {
          return
        }
        if (report === undefined || report.totals.totalOutcomes === 0) {
          setProviderUsage({ status: "empty" })
          return
        }
        setProviderUsage({ status: "loaded", report })
      })
      .catch((error: unknown) => {
        if (!active) {
          return
        }
        if (!(error instanceof Error) && typeof error !== "string") {
          throw error
        }
        setProviderUsage({
          status: "failed",
          message: "Provider usage could not be loaded."
        })
      })

    return () => {
      active = false
    }
  }, [nativeBridge, route, selectedWindowKey])

  return { state: providerUsage, selectedWindowKey, changeWindow }
}
