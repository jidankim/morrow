export const REPORT_JSON = "preflight-report.json";
export const REPORT_MD = "preflight-report.md";
export const PHASE6_DIR = ".omo/evidence/phase-6-cloud-eval-monitoring/final-smoke";

export const REQUIRED_SOURCES = [
  ".omo/plans/wider-product-rollout.md",
  ".omo/drafts/wider-product-rollout.md",
  ".omo/plans/beta-release-distribution.md",
  "package.json",
  "src-tauri/tauri.conf.json",
  "src/domain/appConfig.ts",
  "README.md",
  "docs/axi-phase-0-feedback-eval-baseline.md",
  "docs/plans/beta-release-distribution/README.md",
  "docs/plans/beta-release-distribution/plan.md",
  "docs/plans/beta-release-distribution/draft.md",
  `${PHASE6_DIR}/summary.txt`,
  `${PHASE6_DIR}/preflight-report.json`,
  `${PHASE6_DIR}/release-gate.json`,
  `${PHASE6_DIR}/negative-matrix.json`,
  `${PHASE6_DIR}/privacy-inspect.txt`,
  `${PHASE6_DIR}/cleanup-receipt.txt`
];

export const FORBIDDEN_CONTENT_PATTERNS = [
  { name: "private_home_path", pattern: /\/Users\/[^/\s"]+/ },
  { name: "private_tmp_path", pattern: /\/private\/(?:var|tmp)\// },
  { name: "phase6_canary", pattern: /PHASE6_RAW_CONTENT_CANARY_DO_NOT_STORE/ },
  { name: "morrow_privacy_canary", pattern: /MORROW_PRIVACY_CANARY_RAW/ },
  { name: "openai_secret_key_shape", pattern: /sk-[A-Za-z0-9]{8,}/ },
  { name: "codex_access_secret", pattern: /codex_access_token/i }
];
