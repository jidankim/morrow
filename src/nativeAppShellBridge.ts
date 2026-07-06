import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { z } from "zod"
import type { MenuModel, NativeAppShellState } from "./domain/appShell"

const nativeAppShellStateSchema = z.object({
  mode: z.union([z.literal("scanning"), z.literal("paused"), z.literal("error")]),
  errorMessage: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.string().min(1).optional()
  ),
  onboardingComplete: z.boolean(),
  pendingProposalCount: z.number().int().min(0),
  syncNowRunning: z.boolean(),
  automaticSyncEnabled: z.boolean(),
  automaticSyncStatusLabel: z.union([
    z.literal("Off"),
    z.literal("On"),
    z.literal("Cooling Down"),
    z.literal("Needs Action")
  ]),
  automaticSyncDetail: z.string()
})

export type NativeMenuCommand =
  | "sync-now"
  | "open-settings"
  | "open-calendar"
  | "open-reminders"
  | "toggle-automatic-sync"

const nativeMenuCommandEvents: readonly {
  readonly eventName: string
  readonly command: NativeMenuCommand
}[] = [
  { eventName: "morrow://sync-now", command: "sync-now" },
  { eventName: "morrow://open-settings", command: "open-settings" },
  { eventName: "morrow://open-calendar", command: "open-calendar" },
  { eventName: "morrow://open-reminders", command: "open-reminders" },
  { eventName: "morrow://toggle-automatic-sync", command: "toggle-automatic-sync" }
] as const

export function parseNativeAppShellState(value: unknown): NativeAppShellState {
  return nativeAppShellStateSchema.parse(value)
}

export async function getNativeAppShellState(): Promise<NativeAppShellState> {
  return parseNativeAppShellState(await invoke<unknown>("get_app_state"))
}

export async function setNativeAppShellState(state: NativeAppShellState): Promise<MenuModel> {
  return invoke<MenuModel>("set_app_shell_state", { shellState: parseNativeAppShellState(state) })
}

export async function subscribeNativeAppShellState(
  onState: (state: NativeAppShellState) => void
): Promise<() => void> {
  return listen<unknown>("morrow://app-state", (event) => {
    onState(parseNativeAppShellState(event.payload))
  })
}

export async function subscribeNativeMenuCommand(
  onCommand: (command: NativeMenuCommand) => void
): Promise<() => void> {
  const unsubscribers = await Promise.all(
    nativeMenuCommandEvents.map(({ eventName, command }) =>
      listen(eventName, () => onCommand(command))
    )
  )
  return () => {
    for (const unsubscribe of unsubscribers) {
      unsubscribe()
    }
  }
}
