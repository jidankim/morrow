import { z } from "zod"
import { appConfigSchema } from "./appConfig"
import {
  chatDiscoverySchema,
  selectedChatSchema
} from "./chatDiscovery"
import { syncResultCountsSchema } from "./syncResultCounts"

export const APP_SHELL_STATE_KEY = "morrow.appShellState.v1"

export type ShellStorage = {
  readonly get: (key: string) => string | undefined
  readonly set: (key: string, value: string) => void
}

const appModeSchema = z.union([
  z.literal("scanning"),
  z.literal("paused"),
  z.literal("error")
])
const providerCredentialStatusSchema = z.union([
  z.literal("unchecked"),
  z.literal("configured"),
  z.literal("missing")
])

const appShellStateSchema = z
  .object({
    mode: appModeSchema,
    errorMessage: z.preprocess((value) => (value === null ? undefined : value), z.string().min(1).optional()),
    config: appConfigSchema,
    providerCredentialStatus: providerCredentialStatusSchema.default("unchecked"),
    discovery: chatDiscoverySchema.default({ status: "unverified", chats: [] }),
    selectedChats: z.array(selectedChatSchema)
  })
  .and(syncResultCountsSchema)

export type PersistedAppShellState = z.infer<typeof appShellStateSchema>

export function createBrowserShellStorage(storage: Storage): ShellStorage {
  return {
    get: (key) => storage.getItem(key) ?? undefined,
    set: (key, value) => storage.setItem(key, value)
  }
}

export function loadPersistedAppShellState(
  storage: ShellStorage
): PersistedAppShellState | undefined {
  const stored = storage.get(APP_SHELL_STATE_KEY)
  if (stored === undefined) {
    return undefined
  }
  const parsedJson: unknown = JSON.parse(stored)
  return appShellStateSchema.parse(parsedJson)
}

export function savePersistedAppShellState<TState extends object>(
  state: TState,
  storage: ShellStorage
): void {
  const persistedState = Object.assign({}, state, { providerCredentialStatus: "unchecked" })
  storage.set(
    APP_SHELL_STATE_KEY,
    JSON.stringify(appShellStateSchema.parse(persistedState))
  )
}
