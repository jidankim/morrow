export const scenarios = [
  "Beta Release Handoff",
  "Staged Cohort Expansion",
  "Pause And Resume",
  "Rollback",
  "Default Availability Gate",
  "Privacy Incident Triage",
  "Provider Model Router Incident Triage",
  "Support Escalation",
  "Tester Communication",
  "Evidence Retention",
  "Kill-Switch Use"
]

export const requiredClaims = [
  { name: "live remote config out of scope", phrases: ["Live remote config is out of scope"] },
  { name: "automatic promotion out of scope", phrases: ["Automatic promotion is out of scope"] },
  {
    name: "static manifest reviewed release changes",
    phrases: ["static manifest changed only through reviewed release changes"]
  },
  { name: "gate-only deployment action", phrases: ["deployment_action=none"] },
  {
    name: "manifest update deployment action",
    phrases: ["deployment_action=release_manifest_update"]
  },
  { name: "never silently publish", phrases: ["never silently publish"] },
  {
    name: "signed notarized artifacts required",
    phrases: ["Beta and default availability require signed and notarized artifacts"]
  },
  { name: "telemetry upload disabled", phrases: ["Telemetry and upload paths remain disabled"] },
  { name: "kill switches cannot enable telemetry", phrases: ["must not enable telemetry or upload"] }
]

export const manifestRules = [
  [
    "manifest is non-empty object",
    (manifest) => manifest !== null && !Array.isArray(manifest) && Object.keys(manifest).length > 0
  ],
  ["schema_version wider_product_rollout_manifest_v1", (manifest) => manifest.schema_version === "wider_product_rollout_manifest_v1"],
  ["rollout_source_of_truth", (manifest) => manifest.rollout_source_of_truth === "static_local_manifest"],
  ["remote_config_enabled false", (manifest) => manifest.remote_config_enabled === false],
  ["telemetry_enabled false", (manifest) => manifest.telemetry_enabled === false],
  ["automatic promotion disabled", (manifest) => manifest.promotion?.automatic_promotion_enabled === false],
  ["paused fail-closed state", (manifest) => manifest.promotion?.fail_closed_states?.includes("paused")],
  ["rolled_back fail-closed state", (manifest) => manifest.promotion?.fail_closed_states?.includes("rolled_back")],
  ["blocked fail-closed state", (manifest) => manifest.promotion?.fail_closed_states?.includes("blocked")],
  [
    "default availability requires signed notarized DMG",
    (manifest) => manifest.default_availability?.signed_notarized_dmg_required === true
  ]
]
