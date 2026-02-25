---
name: canisterwallet-js-agent
description: Use this skill when a JavaScript/TypeScript agent needs to call the canisterwalletforagent backend canister (address request, balance, transfer, network discovery, RPC override, and token list management).
---

# CanisterWallet JS Agent Client

Use this skill when building a JS/TS agent that talks to this project's backend canister.

## What this canister provides

- Multi-chain wallet interfaces (address / balance / transfer)
- Network discovery metadata (`wallet_networks`)
- Shared-address grouping (`shared_address_group`) so an agent can infer same-address networks (for example EVM networks)
- Runtime config mutation APIs:
  - RPC overrides (`configured_rpcs`, `set_configured_rpc`, `remove_configured_rpc`)
  - Token list mutation (`configured_tokens`, `add_configured_token`, `remove_configured_token`)

## Important behavior

- Most `*_get_balance_*` methods are `update` calls (not query) because the canister performs HTTP outcalls to external RPC endpoints.
- Address methods are parameterless: `*_request_address()`.
- Runtime config is loaded into canister memory at startup/upgrade from static config and then can be modified by API.
- Method names are strict and use network-name prefixes (snake_case), for example `ethereum_get_balance_eth`, `ton_mainnet_transfer_ton`.

## JS Actor setup

Prefer the generated declarations in this repo:

- `src/declarations/backend/index.js`
- `src/declarations/backend/backend.did.d.ts`

Example (Node.js / JS):

```js
import { HttpAgent } from '@dfinity/agent';
import { createActor } from '../src/declarations/backend/index.js';

const canisterId = process.env.CANISTER_ID_BACKEND;
const host = process.env.IC_HOST ?? 'http://127.0.0.1:4943';

const agent = new HttpAgent({ host });
if (!process.env.IC_NETWORK || process.env.IC_NETWORK !== 'ic') {
  await agent.fetchRootKey();
}

const backend = createActor(canisterId, { agent });
```

## Result decoding pattern (Candid variant)

Most mutating and chain methods return `Result` variants shaped like:

- `{ Ok: ... }`
- `{ Err: ... }`

Use a small helper:

```js
function unwrap(result) {
  if ('Ok' in result) return result.Ok;
  throw new Error(JSON.stringify(result.Err));
}
```

## Discovery-first workflow (recommended)

### 1. Discover available networks and address sharing

```js
const networks = await backend.wallet_networks();
// Each item contains:
// id, primary_symbol, address_family, shared_address_group, supports_balance, supports_send, default_rpc_url
```

Use `shared_address_group` to infer same-address groups instead of hardcoding assumptions.

Example:
- EVM networks share the same `shared_address_group` and therefore the same address string.

### 2. Request an address for a network

Pattern:
- `<network>_request_address()`

Examples:
- `ethereum_request_address()`
- `sepolia_request_address()`
- `bitcoin_request_address()`
- `solana_request_address()`
- `tron_request_address()`
- `ton_mainnet_request_address()`
- `near_mainnet_request_address()`
- `aptos_mainnet_request_address()`
- `sui_mainnet_request_address()`

```js
const addr = unwrap(await backend.ethereum_request_address());
console.log(addr.address, addr.public_key_hex, addr.key_name);
```

### 3. Query balances

Pattern:
- `<network>_get_balance_<asset_kind>(BalanceRequest)`

`BalanceRequest`:
- `account: string` (wallet address / principal text / account identity depending on chain)
- `token: [] | [string]` (optional; token contract / mint / coin type / ledger canister depending on chain)

Examples:

```js
const ethBal = unwrap(await backend.ethereum_get_balance_eth({
  account: '0x...',
  token: [],
}));

const usdcBal = unwrap(await backend.ethereum_get_balance_erc20({
  account: '0x...',
  token: ['0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48'],
}));
```

### 4. Send assets

Pattern:
- `<network>_transfer_<asset_kind>(TransferRequest)`

`TransferRequest` fields:
- `to: string` (required)
- `amount: string` (required, decimal string)
- `token: [] | [string]` (required for token transfers on many chains)
- `from`, `memo`, `nonce`: optional
- `metadata: Array<[string, string]>` optional per-chain extra controls

Example (EVM native):

```js
const tx = unwrap(await backend.ethereum_transfer_eth({
  from: [],
  to: '0xRecipient...',
  amount: '0.001',
  token: [],
  memo: [],
  nonce: [],
  metadata: [],
}));
console.log(tx.tx_id, tx.message);
```

Example (EVM ERC20):

```js
const tx = unwrap(await backend.ethereum_transfer_erc20({
  from: [],
  to: '0xRecipient...',
  amount: '12.5',
  token: ['0xTokenContract...'],
  memo: [],
  nonce: [],
  metadata: [],
}));
```

## Runtime RPC configuration (API-managed)

The canister loads default RPC config into memory at startup and stores runtime overrides in state.

### Read current runtime RPC config

```js
const rpcs = await backend.configured_rpcs();
```

### Set/override a network RPC

```js
unwrap(await backend.set_configured_rpc({
  network: 'ethereum',
  rpc_url: 'https://your-eth-rpc.example',
}));
```

### Remove a runtime RPC override

```js
unwrap(await backend.remove_configured_rpc({
  network: 'ethereum',
}));
// Canister falls back to built-in default RPC for that network.
```

## Token list management (API-managed)

### Read token list for a network

```js
const tokens = await backend.configured_tokens('solana');
```

Returned list includes:
- static tokens (loaded at startup into memory)
- dynamic tokens added via API
- static/dynamic removals filtered by tombstones

### Add token (backend auto-discovers metadata)

```js
const token = unwrap(await backend.add_configured_token({
  network: 'ethereum',
  token_address: '0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48',
}));
```

### Remove token by network + token address

```js
unwrap(await backend.remove_configured_token({
  network: 'ethereum',
  token_address: '0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48',
}));
```

## Network naming rules (important)

Use the backend's canonical network IDs (snake_case), for example:

- `bitcoin`
- `ethereum`
- `sepolia`
- `base`
- `bsc`
- `arbitrum`
- `optimism`
- `avalanche`
- `okx`
- `polygon`
- `internet_computer`
- `solana`
- `solana_testnet`
- `tron`
- `ton_mainnet`
- `near_mainnet`
- `aptos_mainnet`
- `sui_mainnet`

Do not infer API names from coin symbols (`ETH`, `BTC`, `SOL`). Infer from `wallet_networks().id` and append the action/suffix.

## Practical method naming strategy for JS agents

Maintain a small mapping for asset suffixes, then build method names dynamically.

Example starter map:

```js
const primarySuffix = {
  bitcoin: 'btc',
  ethereum: 'eth',
  sepolia: 'eth',
  base: 'eth',
  bsc: 'bnb',
  arbitrum: 'eth',
  optimism: 'eth',
  avalanche: 'avax',
  okx: 'okb',
  polygon: 'pol',
  internet_computer: 'icp',
  solana: 'sol',
  solana_testnet: 'sol',
  tron: 'trx',
  ton_mainnet: 'ton',
  near_mainnet: 'near',
  aptos_mainnet: 'apt',
  sui_mainnet: 'sui',
};

const tokenSuffix = {
  ethereum: 'erc20',
  sepolia: 'erc20',
  base: 'erc20',
  bsc: 'bep20',
  arbitrum: 'erc20',
  optimism: 'erc20',
  avalanche: 'erc20',
  okx: 'erc20',
  polygon: 'erc20',
  internet_computer: 'icrc',
  solana: 'spl',
  solana_testnet: 'spl',
  tron: 'trc20',
  ton_mainnet: 'jetton',
  near_mainnet: 'nep141',
  aptos_mainnet: 'token',
  sui_mainnet: 'token',
};
```

Then call:

- native balance: `${network}_get_balance_${primarySuffix[network]}`
- token balance: `${network}_get_balance_${tokenSuffix[network]}`
- native transfer: `${network}_transfer_${primarySuffix[network]}`
- token transfer: `${network}_transfer_${tokenSuffix[network]}`

## Error handling notes

- `WalletError.InvalidInput` usually indicates malformed address/amount/token parameter.
- `WalletError.Internal` often wraps upstream RPC errors (HTTP outcall, node RPC errors).
- `WalletError.Unimplemented` means the chain/operation path is not implemented yet.
- For `near_mainnet`, an implicit account may not exist on-chain yet; balance may be `0` until funded/initialized.

## Minimal JS agent checklist

1. Create actor from generated declarations.
2. Call `wallet_networks()` and cache by `id`.
3. Use `shared_address_group` for same-address grouping logic.
4. Request address via `${network}_request_address()`.
5. Query balances via network + suffix method naming.
6. Manage token catalogs via `configured_tokens` / `add_configured_token` / `remove_configured_token`.
7. Override RPCs via `set_configured_rpc` when needed.
