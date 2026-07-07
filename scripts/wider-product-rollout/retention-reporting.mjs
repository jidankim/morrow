export function renderMarkdown(report) {
  const lines = [
    "# Todo 5 Retention And Delete All QA",
    "",
    `status: ${report.status}`,
    `generated_at_utc: ${report.generated_at_utc}`,
    `out_dir: ${report.out_dir}`,
    "",
    "## Checks",
  ];
  for (const check of report.checks) {
    lines.push(`- ${check.status} ${check.id}: ${check.observable} (${check.artifact})`);
  }
  if (report.fixture !== undefined) {
    lines.push("", "## Fixture");
    lines.push(`fixture_id: ${report.fixture.fixtureId}`);
    for (const violation of report.fixture.violations) {
      lines.push(`- FAIL ${violation.class}: ${violation.path} ${violation.violation}`);
    }
  }
  lines.push("", "## Runtime Artifacts");
  lines.push("- retention-before.json");
  lines.push("- retention-after.json");
  lines.push("- delete-receipt.json");
  lines.push("- retention-report.json");
  return `${lines.join("\n")}\n`;
}

export function printSummary(checks, reportPath, status) {
  for (const check of checks) {
    console.log(`${check.status} ${check.id}: ${check.observable}`);
  }
  console.log(`retention-report: ${reportPath}`);
  console.log(`RESULT: ${status}`);
}
