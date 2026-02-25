import path from "node:path";
import { fileURLToPath } from "node:url";

import { deployBackendCanister, parsePrincipalList } from "../src/deployer.js";

/**
 * CLI wrapper around deployBackendCanister().
 * It installs/upgrades an existing backend canister.
 *
 * NOTE:
 * - It does not create a production canister with cycles.
 * - Inject a real identity before use in production.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const ENV =
  typeof process !== "undefined" && process?.env ? process.env : Object.create(null);

function resolveWasmPath() {
  const value =
    ENV.WASM_PATH ?? "./target/wasm32-unknown-unknown/release/backend.wasm";
  return path.isAbsolute(value) ? value : path.resolve(REPO_ROOT, value);
}

async function main() {
  const host = ENV.IC_HOST ?? "https://icp-api.io";
  const canisterId = ENV.CANISTER_ID_BACKEND ?? ENV.TARGET_CANISTER_ID;
  const wasmPath = resolveWasmPath();
  const mode = ENV.DEPLOY_MODE ?? "upgrade";
  const initArgHex = ENV.INIT_ARG_HEX ?? "";
  const controllers = parsePrincipalList(ENV.CONTROLLERS ?? "");
  const fetchRootKey = String(ENV.FETCH_ROOT_KEY ?? "").toLowerCase() === "true";

  if (!canisterId) {
    throw new Error(
      "Missing CANISTER_ID_BACKEND (or TARGET_CANISTER_ID). This script upgrades/installs an existing canister.",
    );
  }

  // TODO: inject a real deployment identity.
  const identity = undefined;

  const result = await deployBackendCanister({
    canisterId,
    wasmPath,
    mode,
    initArgHex,
    controllers,
    host,
    identity,
    fetchRootKey,
  });

  console.log("deploy result:", result);
  console.log(
    "Note: production canister creation + cycles funding is not included in this script. Use a wallet/cycles canister first, then run this install/upgrade wrapper.",
  );
}

main().catch((err) => {
  console.error("[deploy-backend-canister] failed");
  console.error(err);
  process.exitCode = 1;
});

