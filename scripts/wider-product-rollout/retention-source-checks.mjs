import { expectIncludes } from "./retention-common.mjs";

export function addSourceChecks(checks, sources) {
  expectIncludes(checks, "config_bounds_schema", "src/domain/appConfig.ts", sources.appConfigText, [
    "localDiagnosticsRetentionDays: z.number().int().min(1).max(365).default(30)",
    "localDiagnosticsRetentionDays: 30",
  ], "1-365 day bounds and default 30 days are encoded in persisted app config");
  expectIncludes(checks, "settings_bounds_ui", "src/SettingsPrivacyControls.tsx", sources.settingsText, [
    "retentionDays < 1 || retentionDays > 365",
    "max={365}",
    "min={1}",
    "Default is 30 days.",
  ], "settings UI constrains local diagnostics retention input to 1-365 days");
  expectIncludes(
    checks,
    "persisted_malformed_rejection_test",
    "src/domain/appShell.persistencePrivacy.test.ts",
    sources.privacyTestText,
    ["rejects malformed local diagnostics retention", "[0, 366, \"30\"]"],
    "persisted settings reject malformed diagnostics retention values"
  );
  expectIncludes(checks, "trace_sink_retention", "crates/morrow-diagnostics/src/sink.rs", sources.sinkText, [
    "const DEFAULT_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);",
    "app_data_dir.join(\"diagnostics\").join(\"traces\")",
    "apply_retention(&traces_dir, config.retention",
    "fs::remove_file(trace_file)?;",
  ], "runtime trace sink uses default 30 day retention under diagnostics/traces");
  expectIncludes(checks, "delete_all_product_scope", "crates/morrow-storage/src/delete_all.rs", sources.deleteAllText, [
    "for artifact_name in [\"traces\", \"evals\", \"exports\"]",
    "let diagnostics_dir = app_data_dir.join(\"diagnostics\");",
    "remove_empty_dir_if_exists(&diagnostics_dir)?;",
  ], "Delete All removes only Morrow-owned diagnostics subdirectories");
  expectIncludes(checks, "delete_receipt_fields", "src-tauri/src/native_bridge/delete_all_protocol.rs", sources.protocolText, [
    "pub diagnostics_artifacts_deleted: bool",
    "pub provider_credential_deletes: Vec<ProviderCredentialDeleteReceipt>",
    "pub provider_oauth_delete_requested: bool",
    "pub provider_oauth_deleted: bool",
  ], "Delete All receipt exposes diagnostics and provider credential fields");
  expectIncludes(checks, "docs_boundary", "docs/diagnostics-trace-eval.md", sources.docsText, [
    "Delete All treats diagnostics artifacts and Morrow-owned legacy provider credentials as private Morrow data.",
    "It removes only Morrow-owned diagnostics subdirectories under the resolved app data root",
    "Delete All leaves the user's global Codex CLI login unchanged.",
  ], "docs state product diagnostics deletion boundary");
  expectIncludes(
    checks,
    "native_delete_scope_test",
    "src-tauri/tests/native_delete/diagnostics.rs",
    sources.nativeDeleteTestText,
    ["delete_all_removes_diagnostics_artifacts", "assert!(sibling_dir.exists());", "diagnosticsArtifactsDeleted"],
    "native Delete All diagnostics test preserves unrelated siblings and checks receipt"
  );
}
