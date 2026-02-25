import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

import { createCanisterFactoryClient } from "../src/canister-factory.js";
import {
  createEncryptedEd25519IdentityFile,
  fileExists,
  loadEncryptedEd25519IdentityFile,
} from "../src/identity-file.js";
import { deployBackendCanister } from "../src/deployer.js";
import { createCanisterWalletClient } from "../src/index.js";

/**
 * OpenClaw self-bootstrap flow (no manual canister creation):
 *
 * 1. Generate encrypted key file (password-protected, no plaintext private key).
 * 2. Load Ed25519 identity from key file.
 * 3. Ask canister-factory canister to create a backend canister for caller (controller = caller/OpenClaw identity).
 * 4. Deploy backend.wasm with that identity.
 * 5. Call rotate_owner(agent_principal) to bootstrap owner in prod mode.
 *
 * Assumptions:
 * - canister-factory canister already exists and has cycles.
 * - canister-factory implements create_canister_for_caller({cycles?}) and uses caller() as controller.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");
const ENV =
  typeof process !== "undefined" && process?.env ? process.env : Object.create(null);

function resolvePath(envValue, fallbackRelative) {
  const value = envValue ?? fallbackRelative;
  return path.isAbsolute(value) ? value : path.resolve(REPO_ROOT, value);
}

async function ensureDirForFile(filePath) {
  await mkdir(path.dirname(filePath), { recursive: true });
}

async function promptPasswordTwice() {
  const rl = createInterface({ input, output });
  try {
    const password = (await rl.question("Enter password for encrypted key file: ")).trim();
    const confirm = (await rl.question("Confirm password: ")).trim();
    if (!password) {
      throw new Error("password is required");
    }
    if (password !== confirm) {
      throw new Error("password confirmation does not match");
    }
    return password;
  } finally {
    rl.close();
  }
}

async function promptPassword() {
  const rl = createInterface({ input, output });
  try {
    const password = (await rl.question("Enter password to unlock key file: ")).trim();
    if (!password) throw new Error("password is required");
    return password;
  } finally {
    rl.close();
  }
}

async function loadState(filePath) {
  try {
    return JSON.parse(await readFile(filePath, "utf8"));
  } catch {
    return {};
  }
}

async function saveState(filePath, next) {
  await ensureDirForFile(filePath);
  await writeFile(filePath, JSON.stringify(next, null, 2) + "\n", "utf8");
}

async function ensureIdentity(identityFilePath) {
  const exists = await fileExists(identityFilePath);
  if (!exists) {
    const password = await promptPasswordTwice();
    await ensureDirForFile(identityFilePath);
    const created = await createEncryptedEd25519IdentityFile({
      filePath: identityFilePath,
      password,
      overwrite: false,
    });
    console.log("Created encrypted identity file:", identityFilePath);
    console.log("OpenClaw principal:", created.principal);
    return created;
  }

  const password = await promptPassword();
  const loaded = await loadEncryptedEd25519IdentityFile({
    filePath: identityFilePath,
    password,
  });
  console.log("Loaded OpenClaw identity principal:", loaded.principal);
  return loaded;
}

async function main() {
  const host = ENV.IC_HOST ?? "https://icp-api.io";
  const wasmPath = resolvePath(
    ENV.WASM_PATH,
    "./target/wasm32-unknown-unknown/release/backend.wasm",
  );
  const identityFilePath = resolvePath(
    ENV.OPENCLAW_KEY_FILE,
    "./.openclaw/openclaw-agent.key.pem",
  );
  const stateFilePath = resolvePath(
    ENV.OPENCLAW_STATE_FILE,
    "./.openclaw/backend-deploy-state.json",
  );
  const canisterFactoryCanisterId =
    ENV.CANISTER_FACTORY_CANISTER_ID ?? ENV.ABC_CANISTER_ID ?? "";
  const canisterFactoryCreateCycles = ENV.CANISTER_FACTORY_CREATE_CYCLES
    ? BigInt(ENV.CANISTER_FACTORY_CREATE_CYCLES)
    : ENV.ABC_CREATE_CYCLES
      ? BigInt(ENV.ABC_CREATE_CYCLES)
      : null;
  const deployMode = ENV.DEPLOY_MODE ?? "install";
  const initArgHex = ENV.INIT_ARG_HEX ?? "";
  const fetchRootKey = String(ENV.FETCH_ROOT_KEY ?? "").toLowerCase() === "true";

  if (!canisterFactoryCanisterId) {
    throw new Error(
      "Missing CANISTER_FACTORY_CANISTER_ID (factory canister id for canister_factory)",
    );
  }

  const { identity, principal } = await ensureIdentity(identityFilePath);
  const state = await loadState(stateFilePath);

  let backendCanisterId = state.backend_canister_id || ENV.CANISTER_ID_BACKEND || "";
  if (!backendCanisterId) {
    console.log(
      "No backend canister id found; requesting one from canister-factory...",
    );
    const canisterFactory = createCanisterFactoryClient({
      canisterId: canisterFactoryCanisterId,
      host,
      identity,
      fetchRootKey,
    });
    backendCanisterId = await canisterFactory.createCanisterForCaller({
      cycles: canisterFactoryCreateCycles,
    });
    console.log("canister-factory created backend canister:", backendCanisterId);
  } else {
    console.log("Using existing backend canister id:", backendCanisterId);
  }

  const deployResult = await deployBackendCanister({
    canisterId: backendCanisterId,
    wasmPath,
    mode: deployMode,
    initArgHex,
    host,
    identity,
    fetchRootKey,
  });
  console.log("deploy result:", deployResult);

  const client = createCanisterWalletClient({
    canisterId: backendCanisterId,
    host,
    identity,
    fetchRootKey,
  });

  try {
    const rotateRes = await client.rotateOwner(principal);
    console.log("rotate_owner result:", rotateRes);
  } catch (err) {
    console.warn(
      "rotate_owner failed (may already be initialized or auth-restricted):",
      err?.message ?? err,
    );
  }

  const serviceInfo = await client.serviceInfo();
  console.log("service_info:", serviceInfo);

  const nextState = {
    ...state,
    canister_factory_canister_id: canisterFactoryCanisterId,
    backend_canister_id: backendCanisterId,
    openclaw_principal: principal,
    last_deploy_mode: deployResult.mode,
    updated_at: new Date().toISOString(),
  };
  await saveState(stateFilePath, nextState);
  console.log("Saved state file:", stateFilePath);
}

main().catch((err) => {
  console.error("[bootstrap-openclaw-backend] failed");
  console.error(err);
  process.exitCode = 1;
});
