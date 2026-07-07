import {
  ALLOWED_STATES,
  FAIL_CLOSED_STATES,
  REQUIRED_KILL_SWITCHES,
} from "./report-constants.mjs";
import { collectForbiddenEntries } from "./report-sensitive.mjs";

export function check(checks, name, passed, details = "") {
  checks.push({
    name,
    status: passed ? "PASS" : "FAIL",
    details,
  });
}

function validateKillSwitches(manifest, checks) {
  const switches = manifest.kill_switches;
  check(
    checks,
    "kill_switches is an object",
    switches !== null && typeof switches === "object" && !Array.isArray(switches),
    `observed ${JSON.stringify(switches)}`,
  );

  if (switches === null || typeof switches !== "object" || Array.isArray(switches)) {
    return false;
  }

  let allPresent = true;
  for (const name of REQUIRED_KILL_SWITCHES) {
    const value = switches[name];
    const present = value !== null && typeof value === "object" && !Array.isArray(value);
    allPresent &&= present;
    check(checks, `kill_switch ${name} is present`, present, `observed ${JSON.stringify(value)}`);

    if (present) {
      check(
        checks,
        `kill_switch ${name} fails closed`,
        value.fail_closed === true,
        `observed ${JSON.stringify(value.fail_closed)}`,
      );
      check(
        checks,
        `kill_switch ${name} is not enabling its path`,
        value.enabled === false,
        `observed ${JSON.stringify(value.enabled)}`,
      );
    }
  }

  return allPresent;
}

function validateDefaultAvailability(manifest, checks) {
  const availability = manifest.default_availability;
  const isObject = availability !== null && typeof availability === "object" && !Array.isArray(availability);
  check(checks, "default_availability is an object", isObject, `observed ${JSON.stringify(availability)}`);
  if (!isObject) {
    return;
  }

  check(
    checks,
    "default availability is limited to static manifest and public docs",
    availability.default_download_channel === "static_manifest_and_public_docs",
    `observed ${JSON.stringify(availability.default_download_channel)}`,
  );
  check(
    checks,
    "signed and notarized DMG is required for default availability",
    availability.signed_notarized_dmg_required === true,
    `observed ${JSON.stringify(availability.signed_notarized_dmg_required)}`,
  );
  check(
    checks,
    "all gates are required for default availability",
    availability.all_gates_required === true,
    `observed ${JSON.stringify(availability.all_gates_required)}`,
  );

  const excluded = Array.isArray(availability.excluded_distribution_channels)
    ? availability.excluded_distribution_channels
    : [];
  for (const channel of [
    "app_store",
    "testflight",
    "auto_updater",
    "hosted_remote_config",
    "automatic_promotion",
  ]) {
    check(
      checks,
      `default availability excludes ${channel}`,
      excluded.includes(channel),
      `observed ${JSON.stringify(excluded)}`,
    );
  }

  if (manifest.state === "default_available") {
    check(
      checks,
      "default_available state requires default availability allowed",
      availability.allowed === true,
      `observed ${JSON.stringify(availability.allowed)}`,
    );
    check(
      checks,
      "default_available state requires all gates passed",
      manifest.gates?.all_required_gates_passed === true,
      `observed ${JSON.stringify(manifest.gates?.all_required_gates_passed)}`,
    );
    check(
      checks,
      "default_available state requires signed artifact",
      manifest.release_artifact?.signing_status === "signed",
      `observed ${JSON.stringify(manifest.release_artifact?.signing_status)}`,
    );
    check(
      checks,
      "default_available state requires notarized artifact",
      manifest.release_artifact?.notarization_status === "notarized",
      `observed ${JSON.stringify(manifest.release_artifact?.notarization_status)}`,
    );
  } else {
    check(
      checks,
      "non-default state does not claim default availability",
      availability.allowed === false,
      `observed ${JSON.stringify(availability.allowed)}`,
    );
  }
}

function validatePromotionModel(manifest, checks) {
  const promotion = manifest.promotion;
  const isObject = promotion !== null && typeof promotion === "object" && !Array.isArray(promotion);
  check(checks, "promotion is an object", isObject, `observed ${JSON.stringify(promotion)}`);
  if (!isObject) {
    return;
  }

  check(
    checks,
    "automatic promotion is disabled",
    promotion.automatic_promotion_enabled === false,
    `observed ${JSON.stringify(promotion.automatic_promotion_enabled)}`,
  );

  const transitions = Array.isArray(promotion.allowed_forward_transitions)
    ? promotion.allowed_forward_transitions.map((transition) => `${transition.from}->${transition.to}`)
    : [];
  check(
    checks,
    "allowed forward transitions are beta->staged and staged->default_available only",
    transitions.length === 2 &&
      transitions.includes("beta->staged") &&
      transitions.includes("staged->default_available"),
    `observed ${JSON.stringify(transitions)}`,
  );

  const failClosed = Array.isArray(promotion.fail_closed_states) ? promotion.fail_closed_states : [];
  for (const state of FAIL_CLOSED_STATES) {
    check(
      checks,
      `fail-closed state ${state} is declared`,
      failClosed.includes(state),
      `observed ${JSON.stringify(failClosed)}`,
    );
  }
}

export function validateManifestObject(manifest) {
  const checks = [];
  const rootIsObject = manifest !== null && typeof manifest === "object" && !Array.isArray(manifest);

  check(
    checks,
    "manifest root is an object",
    rootIsObject,
    rootIsObject ? "observed object" : `observed ${JSON.stringify(manifest)}`,
  );
  if (!rootIsObject) {
    return {
      checks,
      killSwitchFieldsPresent: false,
      forbiddenEntries: [],
    };
  }

  check(
    checks,
    "schema_version is wider_product_rollout_manifest_v1",
    manifest.schema_version === "wider_product_rollout_manifest_v1",
    `observed ${JSON.stringify(manifest.schema_version)}`,
  );
  check(
    checks,
    "rollout_source_of_truth is static_local_manifest",
    manifest.rollout_source_of_truth === "static_local_manifest",
    `observed ${JSON.stringify(manifest.rollout_source_of_truth)}`,
  );
  check(checks, "remote_config_enabled is false", manifest.remote_config_enabled === false, `observed ${JSON.stringify(manifest.remote_config_enabled)}`);
  check(checks, "telemetry_enabled is false", manifest.telemetry_enabled === false, `observed ${JSON.stringify(manifest.telemetry_enabled)}`);
  check(checks, "state is in allowed enum", ALLOWED_STATES.has(manifest.state), `observed ${JSON.stringify(manifest.state)}`);

  const killSwitchFieldsPresent = validateKillSwitches(manifest, checks);
  validatePromotionModel(manifest, checks);
  validateDefaultAvailability(manifest, checks);

  const forbiddenEntries = collectForbiddenEntries(manifest);
  check(
    checks,
    "manifest has no credentials, private paths, or raw private content",
    forbiddenEntries.length === 0,
    `observed ${JSON.stringify(forbiddenEntries)}`,
  );

  return {
    checks,
    killSwitchFieldsPresent,
    forbiddenEntries,
  };
}
