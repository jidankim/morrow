export const requiredClaims = [
  {
    name: "local diagnostics private local files",
    phrases: ["Local diagnostics are private local files", "private local files on the user's Mac"]
  },
  {
    name: "telemetry upload disabled by default",
    phrases: ["Telemetry and upload paths are disabled by default", "not uploaded by default"]
  },
  {
    name: "Delete All scoped to Morrow-owned data",
    phrases: [
      "Delete All removes Morrow-owned diagnostics and Morrow-owned legacy provider markers only"
    ]
  },
  {
    name: "Codex owns user login",
    phrases: ["Codex owns the user login", "Codex owns the login"]
  },
  {
    name: "Morrow does not store provider tokens",
    phrases: ["Morrow does not store provider tokens"]
  },
  {
    name: "Messages and Calendar permissions separate",
    phrases: [
      "Messages and Calendar permissions are separate",
      "Messages Full Disk Access and Calendar access are separate"
    ]
  },
  {
    name: "Phase 6 fixture-backed monitoring only",
    phrases: ["Phase 6 is fixture-backed monitoring only"]
  },
  {
    name: "full live trajectory separately gated",
    phrases: ["The full live Messages-to-Calendar approval trajectory remains separately gated"]
  },
  {
    name: "signed notarized artifacts required",
    phrases: [
      "Beta and default availability require signed and notarized artifacts",
      "signed, notarized DMG"
    ]
  },
  {
    name: "package launch is not live success",
    phrases: ["Package launch by itself only proves that the app opens"]
  }
]

export const forbiddenPatterns = [
  {
    name: "live telemetry upload claim",
    pattern:
      /\b(?:Morrow\s+)?(?:uploads|streams|sends|syncs)\s+(?:live\s+)?(?:telemetry|diagnostics|traces|crash logs)\b(?![^.\n]*(?:disabled|not uploaded|does not|do not|must not|forbidden|blocked))/i
  },
  {
    name: "live cloud collection claim",
    pattern:
      /\b(?:live cloud collection|remote telemetry)\b(?![^.\n]*(?:disabled|not claimed|no live|does not|do not|must not|future work|separate from|not remote telemetry))[^.\n]*(?:enabled|available|complete|proved|proves|ready|uploads|collects)/i
  },
  {
    name: "full live Messages-to-Calendar completion claim",
    pattern:
      /\b(?:proves|proved|verified|passed|completed|completion|success|ships|available|enabled)\b[^.\n]*(?:full live Messages-to-Calendar|full live Messages to Calendar|live Messages-to-Calendar|real Messages-to-Calendar)\b|\b(?:full live Messages-to-Calendar|full live Messages to Calendar|live Messages-to-Calendar|real Messages-to-Calendar)\b(?![^.\n]*(?:separately gated|future work|not prove|does not prove|still requires|not verified|needs|requires))[^.\n]*(?:complete|completed|done|proved|proves|verified|passed|success|ships|available|enabled)/i
  },
  {
    name: "deployed rollout completion claim",
    pattern:
      /\b(?:proves|proved|verified|passed|completed|success|ready|available|enabled)\b[^.\n]*\bdeployed rollout\b|\bdeployed rollout\b(?![^.\n]*(?:future work|not prove|does not prove|not claimed|separate from|blocked|gated))[^.\n]*(?:complete|completed|done|proved|proves|verified|passed|success|ready|available|enabled)/i
  },
  {
    name: "package launch proves real Messages-to-Calendar success",
    pattern:
      /\bpackage launch\b(?![^.\n]*(?:only proves[^.\n]*app opens|does not prove|not prove))[^.\n]*(?:proves|verified|passed|success)[^.\n]*(?:real message|Messages-to-Calendar|Calendar event)/i
  },
  {
    name: "Morrow owns Codex credentials",
    pattern:
      /\bMorrow\b[^.\n]*(?:owns|stores|manages|reads|deletes|logs out|controls)[^.\n]*(?:Codex credentials|Codex login|Codex session|provider tokens)/i
  },
  {
    name: "automatic promotion claim",
    pattern:
      /\bautomatic (?:release )?promotion\b(?![^.\n]*(?:future work|disabled|not claimed|not enabled|does not prove|must not|separate from|blocked))[^.\n]*(?:enabled|available|complete|passed|proves|ready)/i
  },
  {
    name: "diagnostic artifact redistribution as beta",
    pattern:
      /\bdiagnostic artifacts?\b(?![^.\n]*(?:must not be redistributed|not signed\/notarized beta artifacts|not beta release artifacts|not a substitute))[^.\n]*(?:redistributed|shared|published|distributed|released)[^.\n]*(?:as|for)?[^.\n]*\bbeta\b/i
  },
  {
    name: "App Store or TestFlight availability claim",
    pattern:
      /\b(?:available on|released to|shipping on|enabled for|distributed through)\s+(?:the\s+)?(?:App Store|TestFlight)\b|\b(?:App Store|TestFlight)\b[^.\n]*(?:availability is enabled|release is live|distribution is live)/i
  },
  {
    name: "raw private example",
    pattern: /\b(?:raw private example|private message example|real user message|actual user message)\b/i
  },
  {
    name: "raw prompt text",
    pattern: /\b(?:raw prompt:|system prompt:|developer prompt:|prompt transcript:)\b/i
  },
  {
    name: "provider JSON",
    pattern: /(?:\{\s*"provider"\s*:|auth\.json|CODEX_ACCESS_TOKEN|sk-[A-Za-z0-9])/i
  },
  {
    name: "native identifier",
    pattern: /\b(?:message_guid|chat_identifier|ROWID|Z_PK|EventKit identifier|calendarItemIdentifier)\b/i
  },
  {
    name: "app-data path",
    pattern:
      /(?:<app-data>|Application Support\/Morrow|\/Users\/[^ \n]+\/Library\/Application Support|~\/Library\/Application Support)/i
  }
]
