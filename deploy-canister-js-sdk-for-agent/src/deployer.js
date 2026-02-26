import { readFile } from "node:fs/promises";

import { Actor, HttpAgent } from "@dfinity/agent";
import { IDL } from "@dfinity/candid";
import { Principal } from "@dfinity/principal";

const ENV =
  typeof process !== "undefined" && process?.env ? process.env : Object.create(null);

export const MGMT_CANISTER_ID = Principal.fromText("aaaaa-aa");

export function managementIdlFactory({ IDL }) {
  const InstallMode = IDL.Variant({
    install: IDL.Null,
    reinstall: IDL.Null,
    upgrade: IDL.Null,
  });

  const CanisterInstallArgs = IDL.Record({
    mode: InstallMode,
    canister_id: IDL.Principal,
    wasm_module: IDL.Vec(IDL.Nat8),
    arg: IDL.Vec(IDL.Nat8),
  });

  const CanisterSettings = IDL.Record({
    controllers: IDL.Opt(IDL.Vec(IDL.Principal)),
    compute_allocation: IDL.Opt(IDL.Nat),
    memory_allocation: IDL.Opt(IDL.Nat),
    freezing_threshold: IDL.Opt(IDL.Nat),
  });

  const UpdateSettingsArgs = IDL.Record({
    canister_id: IDL.Principal,
    settings: CanisterSettings,
  });

  return IDL.Service({
    install_code: IDL.Func([CanisterInstallArgs], [], []),
    update_settings: IDL.Func([UpdateSettingsArgs], [], []),
  });
}

export function parseDeployMode(value) {
  const mode = String(value ?? "upgrade").trim().toLowerCase();
  if (!["install", "reinstall", "upgrade"].includes(mode)) {
    throw new Error(`Unsupported DEPLOY_MODE: ${mode}`);
  }
  return { [mode]: null };
}

export function parsePrincipalList(csv) {
  if (!csv || !String(csv).trim()) return null;
  const items = String(csv)
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((text) => Principal.fromText(text));
  return items.length ? items : null;
}

export function createManagementActor({
  host = ENV.IC_HOST ?? "https://icp-api.io",
  identity,
  agent,
  fetchRootKey = ENV.DFX_NETWORK !== "ic",
}) {
  const resolvedAgent =
    agent ??
    new HttpAgent({
      host,
      identity,
    });

  if (!agent && fetchRootKey) {
    resolvedAgent.fetchRootKey().catch((err) => {
      console.warn("Unable to fetch root key (expected on local only):", err);
    });
  }

  const actor = Actor.createActor(managementIdlFactory, {
    agent: resolvedAgent,
    canisterId: MGMT_CANISTER_ID,
  });
  return { agent: resolvedAgent, actor };
}

export async function deployBackendCanister({
  canisterId,
  wasmPath,
  mode = "upgrade",
  initArgHex = "",
  controllers = null,
  host,
  identity,
  agent,
  fetchRootKey,
  logger = console,
}) {
  if (!canisterId) {
    throw new Error("canisterId is required");
  }
  if (!wasmPath) {
    throw new Error("wasmPath is required");
  }

  const { actor: mgmt } = createManagementActor({
    host,
    identity,
    agent,
    fetchRootKey,
  });
  const canisterPrincipal =
    canisterId instanceof Principal ? canisterId : Principal.fromText(canisterId);
  const deployMode = typeof mode === "string" ? parseDeployMode(mode) : mode;
  const wasmBytes = new Uint8Array(await readFile(wasmPath));
  const normalizedHex = String(initArgHex ?? "").trim().replace(/^0x/i, "");
  const initArgBytes = normalizedHex
    ? new Uint8Array(Buffer.from(normalizedHex, "hex"))
    : new Uint8Array();
  const parsedControllers = Array.isArray(controllers)
    ? controllers.map((p) => (p instanceof Principal ? p : Principal.fromText(String(p))))
    : null;

  logger.log?.("Deploying backend canister...", {
    canisterId: canisterPrincipal.toText(),
    wasmPath,
    wasmBytes: wasmBytes.length,
    mode: Object.keys(deployMode)[0],
    willUpdateControllers: Boolean(parsedControllers?.length),
  });

  await mgmt.install_code({
    mode: deployMode,
    canister_id: canisterPrincipal,
    wasm_module: [...wasmBytes],
    arg: [...initArgBytes],
  });

  if (parsedControllers?.length) {
    await mgmt.update_settings({
      canister_id: canisterPrincipal,
      settings: {
        controllers: [parsedControllers],
        compute_allocation: [],
        memory_allocation: [],
        freezing_threshold: [],
      },
    });
  }

  return {
    canisterId: canisterPrincipal.toText(),
    mode: Object.keys(deployMode)[0],
    controllers: parsedControllers?.map((p) => p.toText()) ?? null,
  };
}

