#!/usr/bin/env bash
# noqa: SIZE_OK - single-purpose diagnostic release harness keeps OS process, fixture, and reporting boundaries together; split if it grows further.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

evidence_path="${1:-.omo/evidence/task-5-unsigned-adhoc-app-artifact.md}"
evidence_stem="${evidence_path%.*}"
if [ "$evidence_stem" = "$evidence_path" ]; then
  evidence_stem="$evidence_path"
fi
artifact_qa_evidence_path="${MORROW_DIAGNOSTIC_ARTIFACT_QA_EVIDENCE:-${evidence_stem}-artifact-qa.md}"
release_gate_evidence_path="${MORROW_DIAGNOSTIC_RELEASE_GATE_EVIDENCE:-${evidence_stem}-release-gate.md}"
quarantine_evidence_path="${MORROW_DIAGNOSTIC_QUARANTINE_EVIDENCE:-.omo/evidence/task-6-quarantine-unsigned-adhoc-app-artifact.md}"
launch_evidence_path="${MORROW_DIAGNOSTIC_LAUNCH_EVIDENCE:-.omo/evidence/task-6-unsigned-adhoc-app-artifact.md}"
launch_screenshot_path="${MORROW_DIAGNOSTIC_LAUNCH_SCREENSHOT:-.omo/evidence/task-6-unsigned-adhoc-app-artifact.png}"
diagnostic_dir="src-tauri/target/release/diagnostic"
bundle_dir="src-tauri/target/release/bundle"
product_name="$(node -e 'const c=require("./src-tauri/tauri.conf.json"); process.stdout.write(c.productName || "Morrow")')"
version="$(node -e 'const c=require("./src-tauri/tauri.conf.json"); process.stdout.write(c.version || "0.0.0")')"
identifier="$(node -e 'const c=require("./src-tauri/tauri.conf.json"); process.stdout.write(c.identifier || "")')"
host_arch="$(uname -m)"
rust_host_target="$(rustc -vV 2>/dev/null | awk -F': ' '/^host: / { print $2 }' || true)"
rust_host_target="${rust_host_target:-unknown}"
tauri_target="${TAURI_BUILD_TARGET:-host}"
sdkroot_value="${SDKROOT:-/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk}"
rustflags_value="${RUSTFLAGS:--C linker=/Library/Developer/CommandLineTools/usr/bin/cc}"
timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p "$(dirname "$evidence_path")"
exec > "$evidence_path" 2>&1

printf '# Diagnostic Build Evidence\n\n'
printf '## Invocation\n\n`%q %q`\n\n' "$0" "$evidence_path"
printf '## Build Environment\n\n'
printf -- '- APPLE_SIGNING_IDENTITY: `-` (process environment only)\n'
printf -- '- Apple credential hygiene: credential and notarization variables are unset for the Tauri process\n'
printf -- '- SDKROOT: `%s`\n' "$sdkroot_value"
printf -- '- RUSTFLAGS: `%s`\n' "$rustflags_value"
printf -- '- hostArchitecture: `%s`\n' "$host_arch"
printf -- '- rustHostTarget: `%s`\n' "$rust_host_target"
printf -- '- tauriTarget: `%s`\n\n' "$tauri_target"
printf -- '- artifactQaEvidence: `%s`\n' "$artifact_qa_evidence_path"
printf -- '- releaseGateEvidence: `%s`\n' "$release_gate_evidence_path"
printf -- '- quarantineEvidence: `%s`\n' "$quarantine_evidence_path"
printf -- '- launchEvidence: `%s`\n' "$launch_evidence_path"
printf -- '- launchScreenshot: `%s`\n\n' "$launch_screenshot_path"

run_with_diagnostic_apple_env() {
  env \
    -u APPLE_ID \
    -u APPLE_PASSWORD \
    -u APPLE_TEAM_ID \
    -u APPLE_CERTIFICATE \
    -u APPLE_CERTIFICATE_PASSWORD \
    -u APPLE_API_KEY \
    -u APPLE_API_KEY_ID \
    -u APPLE_API_ISSUER \
    -u APPLE_API_ISSUER_ID \
    -u APPLE_API_KEY_PATH \
    -u APPLE_PRIVATE_KEY \
    -u APPLE_PRIVATE_KEY_PATH \
    -u AC_PASSWORD \
    -u ASC_PROVIDER \
    APPLE_SIGNING_IDENTITY="-" \
    SDKROOT="$sdkroot_value" \
    RUSTFLAGS="$rustflags_value" \
    "$@"
}

run_tauri_build() {
  if command -v tauri >/dev/null 2>&1; then
    run_with_diagnostic_apple_env tauri build --bundles app,dmg
  else
    run_with_diagnostic_apple_env npm exec tauri -- build --bundles app,dmg
  fi
}

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
  else
    sha256sum "$1" | awk '{ print $1 }'
  fi
}

copy_path() {
  local source="$1" destination="$2"
  if command -v ditto >/dev/null 2>&1; then
    ditto "$source" "$destination"
  else
    cp -R "$source" "$destination"
  fi
}

repo_relative_path() {
  local path="$1"
  printf '%s' "${path#"$repo_root/"}"
}

repair_diagnostic_launch_metadata() {
  local app_bundle="$1" plist carbon_value
  plist="$app_bundle/Contents/Info.plist"
  if [ ! -f "$plist" ]; then
    printf 'FAIL diagnostic launch metadata: missing `%s`\n' "$plist"
    exit 1
  fi

  carbon_value="$(plutil -extract LSRequiresCarbon raw -o - "$plist" 2>/dev/null || true)"
  if [ "$carbon_value" = "true" ]; then
    plutil -remove LSRequiresCarbon "$plist"
    printf 'PASS diagnostic launch metadata: removed obsolete `LSRequiresCarbon` from Info.plist\n'
  else
    printf 'PASS diagnostic launch metadata: `LSRequiresCarbon` is absent\n'
  fi
}

repair_diagnostic_load_commands() {
  local app_bundle="$1" executable="$2" iconv_dependency
  iconv_dependency="$(otool -L "$executable" 2>/dev/null | awk '/\/nix\/store\/.*libiconv.*\/libiconv\.2\.dylib/ { print $1; exit }')"
  if [ -z "$iconv_dependency" ]; then
    printf 'PASS portable dylib load commands: no Nix libiconv load command found\n'
    return 0
  fi

  if ! command -v install_name_tool >/dev/null 2>&1; then
    printf 'FAIL portable dylib load commands: install_name_tool is required to rewrite `%s`\n' "$iconv_dependency"
    exit 1
  fi
  install_name_tool -change "$iconv_dependency" /usr/lib/libiconv.2.dylib "$executable"
  printf 'PASS portable dylib load commands: rewrote `%s` to `/usr/lib/libiconv.2.dylib`\n' "$iconv_dependency"
  otool -L "$executable"
}

sign_diagnostic_app() {
  local app_bundle="$1"
  codesign --force --deep --sign - --options runtime "$app_bundle"
  printf 'PASS diagnostic app signing: re-signed ad-hoc with hardened runtime option after diagnostic repairs\n'
}

printf '## Tauri Build\n\n'
rm -rf "$diagnostic_dir"
printf 'PASS stale diagnostic output cleanup: removed `%s` before build\n' "$diagnostic_dir"

stale_app_count=0
if [ -d "$bundle_dir" ]; then
  while IFS= read -r stale_app; do
    stale_app_count=$((stale_app_count + 1))
    printf 'PASS stale app source cleanup: removing `%s` before Tauri invocation\n' "$stale_app"
    rm -rf "$stale_app"
  done < <(find "$bundle_dir" -type d -name "${product_name}.app" -prune -print)
fi
printf 'PASS stale app source cleanup: removed %s pre-existing `%s.app` source bundle(s) before Tauri invocation\n' "$stale_app_count" "$product_name"

stale_dmg_count=0
if [ -d "$bundle_dir" ]; then
  while IFS= read -r stale_dmg; do
    stale_dmg_count=$((stale_dmg_count + 1))
    printf 'PASS stale DMG sidecar cleanup: removing `%s` before Tauri invocation\n' "$stale_dmg"
    rm -f "$stale_dmg"
  done < <(find "$bundle_dir" -type f -name '*.dmg' -print)
fi
printf 'PASS stale DMG sidecar cleanup: removed %s pre-existing DMG sidecar(s) before Tauri invocation\n' "$stale_dmg_count"

tauri_invocation_started_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
printf 'PASS current-run freshness baseline: `%s` app sources and DMG sidecars were cleared before Tauri invocation at `%s`\n' "$bundle_dir" "$tauri_invocation_started_at"

set +e
run_tauri_build
tauri_build_status=$?
set -e
if [ "$tauri_build_status" -eq 0 ]; then
  printf 'PASS tauri app/dmg build command: exit 0\n'
else
  printf 'WARN tauri app/dmg build command: exit %s; checking whether optional DMG bundling failed after a valid app was produced\n' "$tauri_build_status"
fi

printf '\n## Staging\n\n'
mkdir -p "$diagnostic_dir/staging"

app_source="$(find "$bundle_dir" -type d -name "${product_name}.app" -prune -print -quit)"
if [ -z "$app_source" ]; then
  printf 'FAIL current-run app freshness: no `%s.app` bundle was created under `%s` by the current Tauri invocation after exit %s\n' "$product_name" "$bundle_dir" "$tauri_build_status"
  exit 1
fi
printf 'PASS current-run app freshness: `%s` was absent before Tauri invocation and exists after Tauri exit %s\n' "$app_source" "$tauri_build_status"

copy_path "$app_source" "$diagnostic_dir/staging/${product_name}.app"
printf 'PASS staged app bundle: `%s`\n' "$diagnostic_dir/staging/${product_name}.app"
repair_diagnostic_launch_metadata "$diagnostic_dir/staging/${product_name}.app"

app_executable="$(find "$diagnostic_dir/staging/${product_name}.app/Contents/MacOS" -type f -perm -111 -print -quit)"
if [ -z "$app_executable" ]; then
  printf 'FAIL app executable discovery: no executable found in staged app\n'
  exit 1
fi
repair_diagnostic_load_commands "$diagnostic_dir/staging/${product_name}.app" "$app_executable"
sign_diagnostic_app "$diagnostic_dir/staging/${product_name}.app"

if [ "$tauri_build_status" -ne 0 ]; then
  if codesign --verify --deep --strict "$diagnostic_dir/staging/${product_name}.app"; then
    printf 'PASS fallback app integrity: staged app verifies after nonzero Tauri exit; optional DMG sidecar will be omitted\n'
  else
    printf 'FAIL fallback app integrity: staged app does not verify after nonzero Tauri exit\n'
    exit 1
  fi
fi

tester_note="$diagnostic_dir/Morrow-diagnostic-tester-note.md"
cat > "$tester_note" <<NOTE
# Morrow Diagnostic Artifact

This is a trusted diagnostic artifact for one technical tester. It is ad-hoc signed, not notarized, host-architecture only, and separate from the official signed/notarized beta release gate.

Host architecture: ${host_arch}
Rust host target: ${rust_host_target}
Tauri target: ${tauri_target}
NOTE

dmg_source=""
if [ "$tauri_build_status" -eq 0 ]; then
  dmg_source="$(find "$bundle_dir" -type f -name '*.dmg' -print -quit || true)"
fi
dmg_name=""
if [ -n "$dmg_source" ]; then
  dmg_name="$(basename "$dmg_source")"
  copy_path "$dmg_source" "$diagnostic_dir/$dmg_name"
  printf 'PASS staged optional DMG sidecar: `%s`\n' "$diagnostic_dir/$dmg_name"
else
  if [ "$tauri_build_status" -eq 0 ]; then
    printf 'PASS optional DMG sidecar: no DMG produced by Tauri\n'
  else
    printf 'PASS optional DMG sidecar: omitted because app packaging succeeded but optional app,dmg command exited %s\n' "$tauri_build_status"
  fi
fi

cp "$tester_note" "$diagnostic_dir/staging/Morrow-diagnostic-tester-note.md"
zip_name="${product_name}-${version}-${host_arch}-diagnostic.zip"
(
  cd "$diagnostic_dir/staging"
  zip -qry "../$zip_name" "${product_name}.app" "Morrow-diagnostic-tester-note.md"
)
printf 'PASS primary zip artifact: `%s`\n' "$diagnostic_dir/$zip_name"

primary_zip_hash="$(sha256 "$diagnostic_dir/$zip_name")"
tester_note_hash="$(sha256 "$tester_note")"
app_executable_hash="$(sha256 "$app_executable")"
dmg_hash=""
if [ -n "$dmg_name" ]; then
  dmg_hash="$(sha256 "$diagnostic_dir/$dmg_name")"
fi

manifest_path="$diagnostic_dir/manifest.json"
git_commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
  worktree_state="dirty-worktree"
else
  worktree_state="clean"
fi
node - "$manifest_path" "$product_name" "$version" "$identifier" "$timestamp" "$git_commit" "$worktree_state" "$host_arch" "$rust_host_target" "$tauri_target" "$zip_name" "$app_executable" "$dmg_name" "$primary_zip_hash" "$tester_note_hash" "$app_executable_hash" "$dmg_hash" "$evidence_path" "$artifact_qa_evidence_path" "$release_gate_evidence_path" "$quarantine_evidence_path" "$launch_evidence_path" "$launch_screenshot_path" "$diagnostic_dir" <<'NODE'
const fs = require("node:fs");
const [
  manifestPath,
  productName,
  version,
  identifier,
  buildTimestamp,
  gitCommit,
  worktreeState,
  hostArchitecture,
  rustHostTarget,
  tauriTarget,
  primaryZip,
  appExecutable,
  dmgSidecar,
  primaryZipHash,
  testerNoteHash,
  appExecutableHash,
  dmgSidecarHash,
  evidencePath,
  artifactQaEvidencePath,
  releaseGateEvidencePath,
  quarantineEvidencePath,
  launchEvidencePath,
  launchScreenshotPath,
  diagnosticDir,
] = process.argv.slice(2);
const manifest = {
  productName,
  version,
  identifier,
  buildTimestamp,
  gitCommit,
  worktreeState,
  signingMode: "ad-hoc",
  notarization: "not_notarized_expected",
  architectureScope: "host_only",
  hostArchitecture,
  rustHostTarget,
  tauriTarget,
  artifacts: {
    primaryZip,
    testerNote: "Morrow-diagnostic-tester-note.md",
    appBundle: `staging/${productName}.app`,
    appExecutable: appExecutable.replace(`${diagnosticDir}/`, ""),
    dmgSidecar: dmgSidecar || null
  },
  freshness: {
    appBundle: "pre_existing_source_bundle_removed_before_tauri_invocation",
    dmgSidecar: "pre_existing_dmg_sidecars_removed_before_tauri_invocation",
    fallbackPolicy: "nonzero_tauri_exit_requires_recreated_current_run_app_bundle"
  },
  sha256: {
    primaryZip: primaryZipHash,
    testerNote: testerNoteHash,
    appExecutable: appExecutableHash,
    dmgSidecar: dmgSidecarHash || null
  },
  evidence: {
    build: evidencePath,
    artifactQa: artifactQaEvidencePath,
    releaseGateRegression: releaseGateEvidencePath,
    quarantineQa: quarantineEvidencePath,
    launchQa: launchEvidencePath,
    launchScreenshot: launchScreenshotPath
  }
};
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
NODE

checksums_path="$diagnostic_dir/SHA256SUMS"
{
  printf '%s  %s\n' "$primary_zip_hash" "$(repo_relative_path "$diagnostic_dir/$zip_name")"
  printf '%s  %s\n' "$(sha256 "$manifest_path")" "$(repo_relative_path "$manifest_path")"
  printf '%s  %s\n' "$tester_note_hash" "$(repo_relative_path "$tester_note")"
  printf '%s  %s\n' "$app_executable_hash" "$(repo_relative_path "$app_executable")"
  if [ -n "$dmg_name" ]; then
    printf '%s  %s\n' "$dmg_hash" "$(repo_relative_path "$diagnostic_dir/$dmg_name")"
  fi
} > "$checksums_path"

handoff_path="$diagnostic_dir/HANDOFF.md"
node - "$handoff_path" "$manifest_path" "$checksums_path" <<'NODE'
const fs = require("node:fs");
const [handoffPath, manifestPath, checksumsPath] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const checksums = fs
  .readFileSync(checksumsPath, "utf8")
  .trim()
  .split(/\n/)
  .map((line) => {
    const match = line.match(/^([a-f0-9]{64})\s+(.+)$/);
    return match ? { hash: match[1], file: match[2] } : null;
  })
  .filter(Boolean);
const evidencePaths = [
  ".omo/evidence/task-3-unsigned-adhoc-app-artifact.md",
  manifest.evidence?.build,
  manifest.evidence?.artifactQa,
  manifest.evidence?.releaseGateRegression,
  manifest.evidence?.quarantineQa,
  manifest.evidence?.launchQa,
  manifest.evidence?.launchScreenshot,
  ".omo/evidence/task-7-unsigned-adhoc-app-artifact.md"
].filter(Boolean);
const dmg = manifest.artifacts?.dmgSidecar || "none";
const lines = [
  "# Morrow Diagnostic Handoff",
  "",
  "This handoff is for a trusted technical tester using the diagnostic artifact.",
  "The diagnostic artifact is not a public beta and must not be redistributed.",
  "",
  "## Artifact Files",
  "",
  `- Primary zip: ${manifest.artifacts.primaryZip}`,
  `- Optional DMG sidecar: ${dmg}`,
  "- Tester instructions: docs/diagnostic-testing.md",
  "- External manifest: manifest.json",
  "- External checksums: SHA256SUMS",
  "",
  "## Architecture",
  "",
  "- Scope: host-architecture only",
  `- Host architecture: ${manifest.hostArchitecture}`,
  `- Rust host target: ${manifest.rustHostTarget}`,
  `- Tauri target: ${manifest.tauriTarget}`,
  "",
  "## Checksums",
  "",
  ...checksums.map(({ hash, file }) => `- ${file}: ${hash}`),
  "",
  "## Signing And Notarization",
  "",
  `- Signing mode: ${manifest.signingMode}`,
  `- Notarization: ${manifest.notarization}`,
  "- Status: ad-hoc signed and not notarized.",
  "- Official release gate remains blocked until Developer ID/notarization credentials exist.",
  "",
  "## Tester Runtime Prerequisites",
  "",
  "- Use Finder right-click Open or System Settings Privacy & Security approval if macOS blocks first launch.",
  "- Grant Full Disk Access to Morrow.app for real Messages discovery.",
  "- Grant Calendar access to Morrow.app for EventKit write/readback QA.",
  "- Codex CLI is a user-owned runtime prerequisite only for provider-backed Sync Now testing.",
  "",
  "## Limitations",
  "",
  "- Host-architecture only; this is not a universal build.",
  "- Package launch only proves that the app opens.",
  "- Real Messages-to-Calendar QA still requires Messages access, provider readiness, Calendar access, and EventKit readback.",
  "- Keep the diagnostic zip, app, note, manifest, checksums, and sidecars limited to the named trusted tester.",
  "",
  "## Evidence",
  "",
  ...evidencePaths.map((path) => `- ${path}`),
  ""
];
fs.writeFileSync(handoffPath, `${lines.join("\n")}\n`);
NODE

printf 'PASS external manifest: `%s`\n' "$manifest_path"
printf 'PASS external SHA256SUMS: `%s`\n' "$checksums_path"
printf 'PASS diagnostic handoff: `%s`\n' "$handoff_path"
printf 'RESULT: PASS\n'
