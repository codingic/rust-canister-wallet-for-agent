import { formatUnits, normalizeAmountResult, parseBigIntLoose } from '../utils/format.js';
import { httpJson } from '../utils/http.js';

function tonApiV3Base(baseUrl) {
  const s = String(baseUrl || '').replace(/\/$/, '');
  return s.endsWith('/api/v2') ? `${s.slice(0, -7)}/api/v3` : s;
}

export async function tonGetBalance(fetchImpl, baseUrl, account) {
  const endpoint = `${String(baseUrl || '').replace(/\/$/, '')}/getAddressBalance?address=${encodeURIComponent(account)}`;
  const parsed = await httpJson(fetchImpl, endpoint);
  const raw = parsed?.result ?? parsed?.balance ?? '0';
  const nano = parseBigIntLoose(raw);
  return normalizeAmountResult({
    network: 'ton_mainnet',
    account,
    amount: formatUnits(nano, 9),
    decimals: 9,
    message: 'ton getAddressBalance'
  });
}

export async function tonGetJettonBalance(fetchImpl, baseUrl, account, jettonMaster) {
  if (!jettonMaster) throw new Error('token (jetton master) is required');
  const v3 = tonApiV3Base(baseUrl);
  const url = `${v3}/jetton/wallets?owner_address=${encodeURIComponent(account)}&jetton_address=${encodeURIComponent(jettonMaster)}&limit=1&offset=0`;
  const parsed = await httpJson(fetchImpl, url);
  const row = Array.isArray(parsed?.jetton_wallets)
    ? parsed.jetton_wallets[0]
    : Array.isArray(parsed?.wallets)
      ? parsed.wallets[0]
      : null;
  if (!row) {
    return normalizeAmountResult({
      network: 'ton_mainnet',
      account,
      token: jettonMaster,
      amount: '0',
      decimals: 0,
      message: 'no jetton wallet found'
    });
  }
  const raw = parseBigIntLoose(row.balance || row.jetton_balance || '0');
  const decimals = Number(row?.jetton?.decimals ?? row?.metadata?.decimals ?? 0);
  return normalizeAmountResult({
    network: 'ton_mainnet',
    account,
    token: jettonMaster,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'ton v3 jetton/wallets'
  });
}
