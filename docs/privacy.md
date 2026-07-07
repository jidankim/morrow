# Privacy

Morrow is designed around local-first scheduling assistance. Public docs should keep the privacy claim narrow: diagnostics are private local files on the user's Mac, telemetry and upload paths are disabled by default, and real Messages-to-Calendar success is only claimed after the separately gated live QA path passes.

## Local Diagnostics

Local diagnostics are private local files. When the local diagnostics setting is enabled, Morrow writes sanitized diagnostic records on the same Mac for troubleshooting and evaluation. These files are not remote telemetry, are not uploaded by default, and do not require Phoenix, Langfuse, LangSmith, Braintrust, LiteLLM, Helicone, or any vendor backend.

Telemetry and crash-log upload settings are hard-off by default. Public claims should not describe a live telemetry pipeline, a cloud upload surface, or redistribution of diagnostic artifacts as beta evidence.

## Delete All

Delete All removes Morrow-owned diagnostics and Morrow-owned legacy provider markers only. It deletes the diagnostics that Morrow created and any old provider markers that Morrow owns. It does not delete unrelated files, user documents, Messages data, Calendar data, or the user's global Codex CLI session.

## Provider Login

Codex owns the user login for the Codex CLI provider path. Morrow can detect whether the CLI is ready and can launch the Codex-owned browser sign-in flow, but Morrow does not store provider tokens and does not read, manage, log out, or delete Codex credentials.

Do not publish provider JSON, prompt transcripts, private message text, native database identifiers, or local storage paths as examples in public docs or release evidence.

## macOS Permissions

Messages and Calendar permissions are separate macOS gates:

- Messages discovery and selected-chat scanning require Full Disk Access for the running Morrow app.
- Calendar proposal creation requires Calendar access for the running Morrow app.

Granting one permission does not grant the other. Provider readiness is separate from both permissions.

## Release Claims

Phase 6 is fixture-backed monitoring only. It covers deterministic fixture-backed cloud eval monitoring inputs and local report artifacts; it does not prove deployed rollout, live cloud collection, provider calls, Messages access, EventKit mutation, or full live Messages-to-Calendar approval completion.

The full live Messages-to-Calendar approval trajectory remains separately gated. Package launch only proves the app opens, not that a real message created a real Calendar event.

Beta and default availability require signed and notarized artifacts. Diagnostic artifacts are not beta release artifacts and must not be redistributed as beta evidence.
