const forbiddenPatterns = [
  { name: "absolute_local_path", pattern: /\/(?:Users|private|var|tmp)\// },
  { name: "raw_provider_shape", pattern: /\bprovider_json\b|\braw_provider\b|\braw_text\b/i },
  { name: "privacy_canary", pattern: /MORROW_PRIVACY_CANARY_RAW/ },
  { name: "auth_material", pattern: /\b(api[_-]?key|secret|token|cookie)\b/i },
  { name: "native_identifier", pattern: /\b(?:chat|message|event|reminder)[_-]?id\b/i },
];

export function assertSanitized(report, markdown) {
  const combined = `${JSON.stringify(report)}\n${markdown}`;
  const hits = forbiddenPatterns
    .filter((entry) => entry.pattern.test(combined))
    .map((entry) => entry.name);
  if (hits.length > 0) {
    return {
      status: "FAIL",
      categories: hits,
    };
  }
  return {
    status: "PASS",
    categories: forbiddenPatterns.map((entry) => entry.name),
  };
}
