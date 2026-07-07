const FORBIDDEN_KEY_PATTERN = /(?:credential|password|secret|token|auth|private[_-]?key)/i;
const FORBIDDEN_VALUE_PATTERN =
  /(?:\/Users\/|\/private\/|~\/\.codex|codex_access_token|auth\.json|provider token|raw message|raw calendar|raw provider)/i;

export function collectForbiddenEntries(value, pointer = "$", findings = []) {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => collectForbiddenEntries(entry, `${pointer}[${index}]`, findings));
    return findings;
  }

  if (value && typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      const childPointer = `${pointer}.${key}`;
      if (FORBIDDEN_KEY_PATTERN.test(key)) {
        findings.push({ pointer: childPointer, reason: "forbidden_sensitive_key" });
      }
      collectForbiddenEntries(entry, childPointer, findings);
    }
    return findings;
  }

  if (typeof value === "string" && FORBIDDEN_VALUE_PATTERN.test(value)) {
    findings.push({ pointer, reason: "forbidden_sensitive_value" });
  }

  return findings;
}
