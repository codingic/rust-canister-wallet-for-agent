import { formatUnits, normalizeAmountResult, parseBigIntLoose } from '../utils/format.js';
import { httpJson } from '../utils/http.js';

async function aptosGetJson(fetchImpl, baseUrl, path) {
  const root = String(baseUrl || '').replace(/\/$/, '');
  return httpJson(fetchImpl, `${root}${path}`);
}

async function aptosPostJson(fetchImpl, baseUrl, path, body) {
  const root = String(baseUrl || '').replace(/\/$/, '');
  return httpJson(fetchImpl, `${root}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
}

function aptosCoinStoreType(coinType) {
  return `0x1::coin::CoinStore<${coinType}>`;
}

async function aptosGetCoinDecimals(fetchImpl, baseUrl, coinType) {
  try {
    const result = await aptosPostJson(fetchImpl, baseUrl, '/view', {
      function: '0x1::coin::decimals',
      type_arguments: [coinType],
      arguments: []
    });
    const v = Array.isArray(result) ? result[0] : null;
    return Number(v ?? 8);
  } catch {
    return 8;
  }
}

export async function aptosGetCoinBalance(fetchImpl, baseUrl, account, coinType, network, tokenValue = '') {
  const typeTag = aptosCoinStoreType(coinType);
  const resource = await aptosGetJson(
    fetchImpl,
    baseUrl,
    `/accounts/${encodeURIComponent(account)}/resource/${encodeURIComponent(typeTag)}`
  );
  const raw = parseBigIntLoose(resource?.data?.coin?.value || '0');
  const decimals = await aptosGetCoinDecimals(fetchImpl, baseUrl, coinType);
  return normalizeAmountResult({
    network,
    account,
    token: tokenValue,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'aptos CoinStore'
  });
}
