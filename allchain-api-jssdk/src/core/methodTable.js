import { EVM_NATIVE_ASSET_KIND, EVM_TOKEN_ASSET_KIND } from '../config/networks.js';
import { normalizeBalanceArgs } from './requests.js';
import { evmGetNativeBalance, evmGetTokenBalance } from '../chains/evm.js';
import { bitcoinGetBalance } from '../chains/bitcoin.js';
import { solanaGetBalance, solanaGetSplBalance } from '../chains/solana.js';
import { tronGetTrxBalance, tronGetTrc20Balance } from '../chains/tron.js';
import { tonGetBalance, tonGetJettonBalance } from '../chains/ton.js';
import { nearGetBalance, nearGetNep141Balance } from '../chains/near.js';
import { aptosGetCoinBalance } from '../chains/aptos.js';
import { suiGetBalance } from '../chains/sui.js';
import {
  internetComputerGetIcpBalance,
  internetComputerGetIcrcBalance
} from '../chains/internetComputer.js';

export function makeMethodTable(ctx) {
  const table = {};

  for (const [network, nativeKind] of Object.entries(EVM_NATIVE_ASSET_KIND)) {
    const tokenKind = EVM_TOKEN_ASSET_KIND[network];
    table[`${network}_get_balance_${nativeKind}`] = async (req) => {
      const { account } = normalizeBalanceArgs(req);
      return evmGetNativeBalance(ctx.fetch, ctx.rpc(network), network, account, 18);
    };
    table[`${network}_get_balance_${tokenKind}`] = async (req) => {
      const { account, token } = normalizeBalanceArgs(req);
      return evmGetTokenBalance(ctx.fetch, ctx.rpc(network), network, account, token);
    };
  }

  table.bitcoin_get_balance_btc = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return bitcoinGetBalance(ctx.fetch, ctx.rpc('bitcoin'), account);
  };

  table.internet_computer_get_balance_icp = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return internetComputerGetIcpBalance(ctx, account);
  };

  table.internet_computer_get_balance_icrc = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return internetComputerGetIcrcBalance(ctx, account, token);
  };

  table.solana_get_balance_sol = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return solanaGetBalance(ctx.fetch, ctx.rpc('solana'), 'solana', account);
  };
  table.solana_get_balance_spl = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return solanaGetSplBalance(ctx.fetch, ctx.rpc('solana'), 'solana', account, token);
  };
  table.solana_testnet_get_balance_sol = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return solanaGetBalance(ctx.fetch, ctx.rpc('solana_testnet'), 'solana_testnet', account);
  };
  table.solana_testnet_get_balance_spl = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return solanaGetSplBalance(ctx.fetch, ctx.rpc('solana_testnet'), 'solana_testnet', account, token);
  };

  table.tron_get_balance_trx = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return tronGetTrxBalance(ctx.fetch, ctx.rpc('tron'), account);
  };
  table.tron_get_balance_trc20 = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return tronGetTrc20Balance(ctx.fetch, ctx.rpc('tron'), account, token);
  };

  table.ton_mainnet_get_balance_ton = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return tonGetBalance(ctx.fetch, ctx.rpc('ton_mainnet'), account);
  };
  table.ton_mainnet_get_balance_jetton = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return tonGetJettonBalance(ctx.fetch, ctx.rpc('ton_mainnet'), account, token);
  };

  table.near_mainnet_get_balance_near = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return nearGetBalance(ctx.fetch, ctx.rpc('near_mainnet'), account);
  };
  table.near_mainnet_get_balance_nep141 = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    return nearGetNep141Balance(ctx.fetch, ctx.rpc('near_mainnet'), account, token);
  };

  table.aptos_mainnet_get_balance_apt = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return aptosGetCoinBalance(
      ctx.fetch,
      ctx.rpc('aptos_mainnet'),
      account,
      '0x1::aptos_coin::AptosCoin',
      'aptos_mainnet',
      ''
    );
  };
  table.aptos_mainnet_get_balance_token = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    if (!token) throw new Error('token (coin type) is required for Aptos token balance');
    return aptosGetCoinBalance(ctx.fetch, ctx.rpc('aptos_mainnet'), account, token, 'aptos_mainnet', token);
  };

  table.sui_mainnet_get_balance_sui = async (req) => {
    const { account } = normalizeBalanceArgs(req);
    return suiGetBalance(ctx.fetch, ctx.rpc('sui_mainnet'), account, '', 'sui_mainnet');
  };
  table.sui_mainnet_get_balance_token = async (req) => {
    const { account, token } = normalizeBalanceArgs(req);
    if (!token) throw new Error('token (coin type) is required for Sui token balance');
    return suiGetBalance(ctx.fetch, ctx.rpc('sui_mainnet'), account, token, 'sui_mainnet');
  };

  return table;
}
