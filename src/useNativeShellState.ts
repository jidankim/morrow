import { useEffect, type Dispatch, type SetStateAction } from "react"
import {
  applyNativeAppShellState,
  reduceAppShellState,
  type AppShellState,
  type NativeAppShellState
} from "./domain/appShell"
import { nativeErrorMessage } from "./nativeErrors"
import type { NativeShellBridge } from "./tauriBridge"

type NativeStateSource = "initial" | "event"

const shouldApplyNativeState = (
  current: AppShellState,
  nativeState: NativeAppShellState,
  source: NativeStateSource
): boolean => source === "event" || nativeState.mode !== "scanning" || current.mode === "scanning"

export function useNativeShellState(
  nativeBridge: NativeShellBridge,
  setState: Dispatch<SetStateAction<AppShellState>>
): void {
  useEffect(() => {
    let active = true
    let unsubscribe: (() => void) | undefined

    const applyNativeState = (nativeState: NativeAppShellState, source: NativeStateSource): void => {
      setState((current) => {
        if (!shouldApplyNativeState(current, nativeState, source)) {
          return current
        }
        return applyNativeAppShellState(current, nativeState)
      })
    }

    void nativeBridge
      .getState()
      .then((nativeState) => {
        if (active && nativeState !== undefined) {
          applyNativeState(nativeState, "initial")
        }
        return nativeBridge.subscribeAppState((nativeState) => {
          if (active) {
            applyNativeState(nativeState, "event")
          }
        })
      })
      .then((nextUnsubscribe) => {
        if (active) {
          unsubscribe = nextUnsubscribe
          return
        }
        nextUnsubscribe?.()
      })
      .catch((error: unknown) => {
        const message = nativeErrorMessage(error, "Native app shell state could not be read.")
        setState((current) =>
          reduceAppShellState(current, {
            type: "fail",
            message
          })
        )
      })

    return () => {
      active = false
      unsubscribe?.()
    }
  }, [nativeBridge, setState])
}
