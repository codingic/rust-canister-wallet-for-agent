import { formatUnits, normalizeAmountResult, parseBigIntLoose } from '../utils/format.js';
import { jsonRpc } from '../utils/http.js';

export async function suiGetBalance(fetchImpl, rpcUrl, account, coinType, network) {
  const params = coinType ? [account, coinType] : [account];
  const result = await jsonRpc(fetchImpl, rpcUrl, 'suix_getBalance', params);
  const raw = parseBigIntLoose(result?.totalBalance || '0');
  const decimals = Number(result?.decimals ?? (coinType ? 0 : 9));
  return normalizeAmountResult({
    network,
    account,
    token: coinType || '',
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'sui suix_getBalance'
  });
}
