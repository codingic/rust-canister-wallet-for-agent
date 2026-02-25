import { Actor, HttpAgent } from "@dfinity/agent";
import { IDL } from "@dfinity/candid";
import { Principal } from "@dfinity/principal";

const ENV =
  typeof process !== "undefined" && process?.env ? process.env : Object.create(null);

// Expected canister-factory API (recommended):
// create_canister_for_caller : (record { cycles : opt nat64 }) -> (variant { Ok : principal; Err : text })
export function canisterFactoryIdlFactory({ IDL }) {
  const CreateCanisterForCallerRequest = IDL.Record({
    cycles: IDL.Opt(IDL.Nat64),
  });
  const CreateCanisterForCallerError = IDL.Variant({
    Internal: IDL.Text,
    QuotaExceeded: IDL.Text,
    Forbidden: IDL.Text,
    InvalidInput: IDL.Text,
  });
  const CreateCanisterForCallerResult = IDL.Variant({
    Ok: IDL.Principal,
    Err: CreateCanisterForCallerError,
  });

  return IDL.Service({
    create_canister_for_caller: IDL.Func(
      [CreateCanisterForCallerRequest],
      [CreateCanisterForCallerResult],
      [],
    ),
  });
}

export function createCanisterFactoryClient({
  canisterId = ENV.CANISTER_FACTORY_CANISTER_ID ?? ENV.ABC_CANISTER_ID,
  host,
  identity,
  agent,
  fetchRootKey = ENV.DFX_NETWORK !== "ic",
}) {
  if (!canisterId) {
    throw new Error(
      "canister-factory canisterId is required (CANISTER_FACTORY_CANISTER_ID)",
    );
  }

  const resolvedAgent =
    agent ??
    new HttpAgent({
      host,
      identity,
    });

  if (!agent && fetchRootKey) {
    resolvedAgent.fetchRootKey().catch((err) => {
      console.warn("Unable to fetch root key for canister-factory client");
      console.error(err);
    });
  }

  const actor = Actor.createActor(canisterFactoryIdlFactory, {
    agent: resolvedAgent,
    canisterId: Principal.fromText(canisterId),
  });

  return {
    agent: resolvedAgent,
    actor,
    async createCanisterForCaller({ cycles = null } = {}) {
      const res = await actor.create_canister_for_caller({
        cycles: cycles == null ? [] : [BigInt(cycles)],
      });
      if ("Ok" in res) {
        return res.Ok.toText();
      }
      throw new Error(
        `canister_factory.create_canister_for_caller failed: ${JSON.stringify(res.Err)}`,
      );
    },
  };
}
