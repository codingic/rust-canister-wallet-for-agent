# allchain-api-jssdk

JS SDK for chain-side balance queries.

This SDK exposes methods using the same naming pattern as the removed backend balance APIs:

`<network_prefix>_get_balance_<asset_kind>`

Examples:
- `ethereum_get_balance_eth`
- `ethereum_get_balance_erc20`
- `bitcoin_get_balance_btc`
- `solana_get_balance_spl`
- `internet_computer_get_balance_icp`

## Usage

```js
import { createAllChainApiClient } from './src/index.js';

const client = createAllChainApiClient({
  rpc: {
    ethereum: 'https://ethereum-rpc.publicnode.com',
    solana: 'https://api.mainnet-beta.solana.com'
  },
  icp: {
    host: 'https://icp-api.io',
    ledgerCanisterId: 'ryjl3-tyaaa-aaaaa-aaaba-cai'
  }
});

const eth = await client.ethereum_get_balance_eth({
  account: '0x0000000000000000000000000000000000000000'
});

const erc20 = await client.ethereum_get_balance_erc20({
  account: '0x0000000000000000000000000000000000000000',
  token: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606EB48'
});
```

## Return shape

Methods return a normalized object similar to the backend `BalanceResponse` fields:

```js
{
  network, account, token, amount, decimals,
  pending: false,
  blockRef: null,
  message
}
```

## Notes

- `internet_computer_get_balance_icp` / `icrc` use `@dfinity/agent` + ICRC calls.
- `tron_get_balance_trc20` supports TRON addresses in `T...` or `41...` hex format.
- `ton_mainnet_get_balance_jetton` expects a TON API endpoint compatible with the configured path.
