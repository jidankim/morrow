import path from "node:path";

import { addCheck, planningEvidencePrefix, readText } from "./retention-common.mjs";

export function evaluateFixture(repoRoot, fixturePath) {
  const fixture = JSON.parse(readText(repoRoot, fixturePath));
  const proposedDeletedPaths = Array.isArray(fixture.proposedDeletedPaths) ? fixture.proposedDeletedPaths : [];
  const normalizedPaths = proposedDeletedPaths.map((item) => String(item).replace(/^[.][/]/, ""));
  const violations = normalizedPaths
    .filter((item) => item === ".omo/evidence" || item.startsWith(planningEvidencePrefix))
    .map((item) => ({
      class: "planning-evidence",
      path: item,
      violation: "Delete All must not product-delete .omo/evidence planning artifacts",
    }));
  return { fixtureId: fixture.fixtureId ?? path.basename(fixturePath), proposedDeletedPaths, violations };
}

export function addFixtureChecks(checks, fixtureResult, fixturePath) {
  for (const violation of fixtureResult.violations) {
    addCheck(
      checks,
      "FAIL",
      "fixture_planning_evidence_violation",
      "malformed fixture represents Delete All deleting .omo/evidence and must fail",
      `${violation.class} violation: ${violation.path}`,
      fixturePath,
      violation
    );
  }
  if (fixtureResult.violations.length === 0) {
    addCheck(
      checks,
      "PASS",
      "fixture_has_no_planning_evidence_violation",
      "fixture does not propose product-deleting planning evidence",
      "no .omo/evidence paths proposed",
      fixturePath
    );
  }
}
