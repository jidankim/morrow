import { useCallback } from "react"
import { privacyPaneName } from "./domain/privacySettings"
import type { NativeShellBridge, PrivacySettingsPane } from "./tauriBridge"

export function usePrivacySettingsOpener(
  nativeBridge: NativeShellBridge
): (pane: PrivacySettingsPane) => Promise<void> {
  return useCallback(
    async (pane: PrivacySettingsPane): Promise<void> => {
      const receipt = await nativeBridge.openPrivacySettings({ pane })
      if (receipt === undefined || !receipt.opened) {
        throw new Error(`${privacyPaneName(pane)} could not be opened.`)
      }
    },
    [nativeBridge]
  )
}
