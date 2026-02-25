import { Actor, HttpAgent } from "@dfinity/agent";
import { idlFactory } from "./backend.did.js";

const ENV =
  typeof process !== "undefined" && process?.env ? process.env : Object.create(null);

export const NETWORKS = Object.freeze({
  BITCOIN: "bitcoin",
  ETHEREUM: "ethereum",
  SEPOLIA: "sepolia",
  BASE: "base",
  BSC: "bsc",
  ARBITRUM: "arbitrum",
  OPTIMISM: "optimism",
  AVALANCHE: "avalanche",
  OKX: "okx",
  POLYGON: "polygon",
  INTERNET_COMPUTER: "internet_computer",
  SOLANA: "solana",
  SOLANA_TESTNET: "solana_testnet",
  TRON: "tron",
  TON_MAINNET: "ton_mainnet",
  NEAR_MAINNET: "near_mainnet",
  APTOS_MAINNET: "aptos_mainnet",
  SUI_MAINNET: "sui_mainnet",
});

export const PRIMARY_ASSET_SUFFIX = Object.freeze({
  [NETWORKS.BITCOIN]: "btc",
  [NETWORKS.ETHEREUM]: "eth",
  [NETWORKS.SEPOLIA]: "eth",
  [NETWORKS.BASE]: "eth",
  [NETWORKS.BSC]: "bnb",
  [NETWORKS.ARBITRUM]: "eth",
  [NETWORKS.OPTIMISM]: "eth",
  [NETWORKS.AVALANCHE]: "avax",
  [NETWORKS.OKX]: "okb",
  [NETWORKS.POLYGON]: "pol",
  [NETWORKS.INTERNET_COMPUTER]: "icp",
  [NETWORKS.SOLANA]: "sol",
  [NETWORKS.SOLANA_TESTNET]: "sol",
  [NETWORKS.TRON]: "trx",
  [NETWORKS.TON_MAINNET]: "ton",
  [NETWORKS.NEAR_MAINNET]: "near",
  [NETWORKS.APTOS_MAINNET]: "apt",
  [NETWORKS.SUI_MAINNET]: "sui",
});

export const TOKEN_ASSET_SUFFIX = Object.freeze({
  [NETWORKS.ETHEREUM]: "erc20",
  [NETWORKS.SEPOLIA]: "erc20",
  [NETWORKS.BASE]: "erc20",
  [NETWORKS.BSC]: "bep20",
  [NETWORKS.ARBITRUM]: "erc20",
  [NETWORKS.OPTIMISM]: "erc20",
  [NETWORKS.AVALANCHE]: "erc20",
  [NETWORKS.OKX]: "erc20",
  [NETWORKS.POLYGON]: "erc20",
  [NETWORKS.INTERNET_COMPUTER]: "icrc",
  [NETWORKS.SOLANA]: "spl",
  [NETWORKS.SOLANA_TESTNET]: "spl",
  [NETWORKS.TRON]: "trc20",
  [NETWORKS.TON_MAINNET]: "jetton",
  [NETWORKS.NEAR_MAINNET]: "nep141",
  [NETWORKS.APTOS_MAINNET]: "token",
  [NETWORKS.SUI_MAINNET]: "token",
});

function normalizeNetworkName(network) {
  if (!network) return "";
  return String(network).trim().toLowerCase().replace(/-/g, "_");
}

function toOpt(value) {
  return value === undefined || value === null || value === "" ? [] : [value];
}

function fromOpt(opt) {
  return Array.isArray(opt) ? (opt.length ? opt[0] : null) : opt ?? null;
}

export function unwrapResult(result) {
  if (result && typeof result === "object") {
    if ("Ok" in result) return result.Ok;
    if ("Err" in result) {
      const err = new Error(typeof result.Err === "string" ? result.Err : JSON.stringify(result.Err));
      err.walletError = result.Err;
      throw err;
    }
  }
  return result;
}

export function buildRequestAddressMethod(network) {
  const n = normalizeNetworkName(network);
  if (!n) throw new Error("network is required");
  return `${n}_request_address`;
}

export function buildBalanceMethod(network, { token } = {}) {
  const n = normalizeNetworkName(network);
  if (!n) throw new Error("network is required");
  const suffix = token ? TOKEN_ASSET_SUFFIX[n] : PRIMARY_ASSET_SUFFIX[n];
  if (!suffix) {
    throw new Error(
      token
        ? `token balance is not supported or not mapped for network: ${n}`
        : `native balance is not supported or not mapped for network: ${n}`,
    );
  }
  return `${n}_get_balance_${suffix}`;
}

export function buildTransferMethod(network, { token } = {}) {
  const n = normalizeNetworkName(network);
  if (!n) throw new Error("network is required");
  const suffix = token ? TOKEN_ASSET_SUFFIX[n] : PRIMARY_ASSET_SUFFIX[n];
  if (!suffix) {
    throw new Error(
      token
        ? `token transfer is not supported or not mapped for network: ${n}`
        : `native transfer is not supported or not mapped for network: ${n}`,
    );
  }
  return `${n}_transfer_${suffix}`;
}

export function createCanisterWalletClient(options = {}) {
  const {
    actor,
    agent,
    identity,
    host,
    canisterId = ENV.CANISTER_ID_BACKEND,
    actorOptions,
    agentOptions = {},
    fetchRootKey = ENV.DFX_NETWORK !== "ic",
  } = options;

  if (!actor && !canisterId) {
    throw new Error("canisterId is required when actor is not provided");
  }

  const resolvedAgent =
    agent ??
    new HttpAgent({
      host,
      identity,
      ...agentOptions,
    });

  if (!actor && fetchRootKey) {
    resolvedAgent.fetchRootKey().catch((err) => {
      console.warn("Unable to fetch root key; ensure local replica is running");
      console.error(err);
    });
  }

  const backend =
    actor ??
    Actor.createActor(idlFactory, {
      agent: resolvedAgent,
      canisterId,
      ...(actorOptions ?? {}),
    });

  async function callResult(method, ...args) {
    const fn = backend?.[method];
    if (typeof fn !== "function") {
      throw new Error(`backend actor method not found: ${method}`);
    }
    return unwrapResult(await fn(...args));
  }

  async function call(method, ...args) {
    const fn = backend?.[method];
    if (typeof fn !== "function") {
      throw new Error(`backend actor method not found: ${method}`);
    }
    return fn(...args);
  }

  return {
    actor: backend,

    normalizeNetworkName,
    unwrapResult,
    buildRequestAddressMethod,
    buildBalanceMethod,
    buildTransferMethod,

    async whoami() {
      return call("whoami");
    },

    async serviceInfo() {
      return call("service_info");
    },

    async walletNetworks() {
      return call("wallet_networks");
    },

    async supportedNetworks() {
      return call("supported_networks");
    },

    async sharedAddressGroups() {
      const networks = await call("wallet_networks");
      const groups = new Map();
      for (const item of networks) {
        const key = item.shared_address_group;
        const existing = groups.get(key) ?? [];
        existing.push(item.id);
        groups.set(key, existing);
      }
      return Object.fromEntries(groups);
    },

    async requestAddress(network) {
      const method = buildRequestAddressMethod(network);
      return callResult(method);
    },

    async getBalance({ network, account, token = null }) {
      const method = buildBalanceMethod(network, { token });
      return callResult(method, {
        account,
        token: toOpt(token),
      });
    },

    async transfer({
      network,
      to,
      amount,
      token = null,
      from = null,
      memo = null,
      nonce = null,
      metadata = [],
    }) {
      const method = buildTransferMethod(network, { token });
      return callResult(method, {
        from: toOpt(from),
        to,
        amount: String(amount),
        token: toOpt(token),
        memo: toOpt(memo),
        nonce: toOpt(nonce),
        metadata,
      });
    },

    async configuredExplorer(network) {
      const result = await call("configured_explorer", normalizeNetworkName(network));
      return fromOpt(result);
    },

    async configuredTokens(network) {
      return call("configured_tokens", normalizeNetworkName(network));
    },

    async addConfiguredToken({ network, tokenAddress }) {
      return callResult("add_configured_token", {
        network: normalizeNetworkName(network),
        token_address: tokenAddress,
      });
    },

    async removeConfiguredToken({ network, tokenAddress }) {
      return callResult("remove_configured_token", {
        network: normalizeNetworkName(network),
        token_address: tokenAddress,
      });
    },

    async configuredRpcs() {
      return call("configured_rpcs");
    },

    async setConfiguredRpc({ network, rpcUrl }) {
      return callResult("set_configured_rpc", {
        network: normalizeNetworkName(network),
        rpc_url: rpcUrl,
      });
    },

    async removeConfiguredRpc({ network }) {
      return callResult("remove_configured_rpc", {
        network: normalizeNetworkName(network),
      });
    },

    async pause() {
      return callResult("pause");
    },

    async unpause() {
      return callResult("unpause");
    },

    async rotateOwner(principal) {
      return callResult("rotate_owner", principal);
    },

    async raw(method, ...args) {
      return call(method, ...args);
    },

    async rawResult(method, ...args) {
      return callResult(method, ...args);
    },
  };
}

export default createCanisterWalletClient;
export { idlFactory };
