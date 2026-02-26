import { formatUnits, normalizeAmountResult } from '../utils/format.js';
import { httpJson } from '../utils/http.js';

export async function bitcoinGetBalance(fetchImpl, apiBase, account) {
  const normalized = String(apiBase || '').replace(/\/$/, '');
  const data = await httpJson(fetchImpl, `${normalized}/address/${account}`);
  const cs = data?.chain_stats || {};
  const ms = data?.mempool_stats || {};
  const funded = BigInt(cs.funded_txo_sum || 0) + BigInt(ms.funded_txo_sum || 0);
  const spent = BigInt(cs.spent_txo_sum || 0) + BigInt(ms.spent_txo_sum || 0);
  const sats = funded - spent;
  return normalizeAmountResult({
    network: 'bitcoin',
    account,
    amount: formatUnits(sats, 8),
    decimals: 8,
    message: 'esplora address'
  });
}
