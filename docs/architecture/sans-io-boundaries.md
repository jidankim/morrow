# Sans-IO Boundary Map

Status: Tasks 1-9 have been independently confirmed in start-work evidence. Final F1-F4/global verification is still pending, so this note records the current boundary shape but does not claim the whole plan complete.

## Topology

| ID | Functional core | Imperative adapter | Do not move into core |
| --- | --- | --- | --- |
| `C1-ui-shell-contracts` | TypeScript state contracts, zod parsing, sync result count parsing, and command request shaping. | `NativeShellBridge`, Tauri runtime detection, `invoke`/`listen`, and UI state subscriptions. | Tauri invoke/listen, browser runtime checks, React/UI state runtime effects, menu event subscriptions, token/keychain command effects. |
| `C2-scan-use-case-core` | Scan outcome planning, privacy-safe persistence intents, cap/replay selection, request/config derivation, and result count mapping. | `scan_selected_chats_with_dependencies` opens storage, ingests Messages, runs provider-backed detection, applies caps, persists outcomes, replays proposals, and reads feedback counts. | Messages reads, provider calls, `Store::open`, sqlite persistence, proposal adapter calls, filesystem/app paths, and native error plumbing. |
| `C3-messages-adapters` | `morrow-messages` typed values: `ChatGuid`, `MessageGuid`, `NativeBatch`, `MessageEvidence`, ingestion reports, and `MessagesDataSource` trait contract. | Messages SQLite/native adapters implement discovery and reads with protected DB access and row parsing. | Messages SQLite/protected DB access, sqlite CLI invocation, Full Disk Access handling, SQL strings, native permission mapping, and raw macOS Messages paths. |
| `C4-detection-provider-boundary` | Detection parser/pipeline contracts, provider request/response types, schema validation, and fake-provider-testable extraction flow. | OpenAI and Codex providers supply transport, credentials, payload delivery, subprocess/network behavior, and response retrieval. | OpenAI/Codex transport, API keys/keychain, network, subprocess runners, model/prompt policy, timeout/auth behavior, and token redaction surfaces. |
| `C5-reconcile-proposal-replay` | `reconcile_candidate` planning and proposal replay decision helpers for mapping-present/create/finalize/failure transitions. | EventKit/Reminders proposal adapters, external object creation, store transitions, recovery marking, and failure persistence. | EventKit/Reminders calls, external calendar/reminder mutation, `Store` state transitions, durable mapping writes, and native proposal adapter details. |
| `C6-storage-durability-edge` | Typed persistence intents and storage-facing decisions that can be tested without opening a database. | `Store`, sqlite CLI, migrations, visibility caps, feedback eval counts, quiet logs, candidate writes, and durable external mappings. | sqlite CLI/Store persistence, schema/migration/backend choices, DB file paths, app data paths, durability checks, and cleanup/delete effects. |

## Boundary Rules

- Keep the Functional core deterministic: given typed input, it returns typed decisions, plans, counts, reports, or validation errors without touching native services.
- Keep each Imperative adapter thin: parse at the edge, call the native service or persistence layer, convert failures into existing error contracts, and delegate decisions back to the core.
- Do not move into core: Tauri invoke/listen, EventKit/Reminders, Messages SQLite/protected DB access, sqlite CLI/Store persistence, OpenAI/Codex transport/keychain/network/subprocess, filesystem/app paths, or UI state runtime effects.
- Future work must preserve current Tauri command names, zod schemas, database schema, provider model/prompt behavior, privacy behavior, and user-visible UI behavior unless explicitly scoped.
