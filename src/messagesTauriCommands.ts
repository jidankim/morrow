import { invoke } from "@tauri-apps/api/core"
import { syncResultCountsSchema, type SyncResultCounts } from "./domain/syncResultCounts"
import {
  parseMessagesChatPreviewReport,
  parseMessagesChatPreviewRequest,
  parseMessagesDiscoveryReport,
  parseSyncScanRequest,
  type MessagesChatPreviewReport,
  type MessagesChatPreviewRequest,
  type MessagesDiscoveryReport,
  type SyncScanRequest
} from "./messagesDiscoveryBridge"

export type SyncScanResult = SyncResultCounts

function parseSyncScanResult(value: unknown): SyncScanResult {
  return syncResultCountsSchema.parse(value)
}

export async function scanSelectedChatsInTauri(
  request: SyncScanRequest
): Promise<SyncScanResult> {
  const result = await invoke<unknown>("scan_selected_chats", { request: parseSyncScanRequest(request) })
  return parseSyncScanResult(result)
}

export async function discoverMessagesChatsInTauri(): Promise<MessagesDiscoveryReport> {
  const report = await invoke<unknown>("discover_messages_chats")
  return parseMessagesDiscoveryReport(report)
}

export async function loadMessagesChatPreviewsInTauri(
  request: MessagesChatPreviewRequest
): Promise<MessagesChatPreviewReport> {
  const report = await invoke<unknown>("load_messages_chat_previews", {
    request: parseMessagesChatPreviewRequest(request)
  })
  return parseMessagesChatPreviewReport(report)
}
