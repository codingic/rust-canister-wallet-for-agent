import bs58 from 'bs58';
import { formatUnits, normalizeAmountResult, stripHexPrefix } from '../utils/format.js';
import { httpJson } from '../utils/http.js';

function tronBase58ToHex(address) {
  const decoded = bs58.decode(address);
  if (decoded.length !== 25) throw new Error('invalid tron base58 address length');
  const payload = decoded.subarray(0, 21);
  if (typeof Buffer !== 'undefined') return Buffer.from(payload).toString('hex');
  return Array.from(payload)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function normalizeTronHexAddress(address) {
  const raw = String(address || '').trim();
  if (!raw) throw new Error('empty tron address');
  if (/^T[1-9A-HJ-NP-Za-km-z]+$/.test(raw)) return tronBase58ToHex(raw);
  const hex = stripHexPrefix(raw);
  if (/^[0-9a-fA-F]{42}$/.test(hex)) return hex;
  throw new Error('invalid tron address format (use base58 T... or 41-prefixed hex)');
}

function pad32Hex(hexNoPrefix) {
  return String(hexNoPrefix || '').padStart(64, '0');
}

function tronAbiEncodeAddressParam(address) {
  const hex = normalizeTronHexAddress(address);
  const evm20 = hex.slice(2);
  return pad32Hex(evm20.toLowerCase());
}

async function tronPost(fetchImpl, baseUrl, path, body) {
  return httpJson(fetchImpl, `${String(baseUrl || '').replace(/\/$/, '')}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
}

export async function tronGetTrxBalance(fetchImpl, baseUrl, account) {
  const result = await tronPost(fetchImpl, baseUrl, '/wallet/getaccount', {
    address: account,
    visible: true
  });
  const sun = BigInt(result?.balance || 0);
  return normalizeAmountResult({
    network: 'tron',
    account,
    amount: formatUnits(sun, 6),
    decimals: 6,
    message: 'tron wallet/getaccount'
  });
}

export async function tronGetTrc20Balance(fetchImpl, baseUrl, account, token) {
  if (!token) throw new Error('token is required for TRC20 balance');
  const [bal, dec] = await Promise.all([
    tronPost(fetchImpl, baseUrl, '/wallet/triggerconstantcontract', {
      owner_address: account,
      contract_address: token,
      function_selector: 'balanceOf(address)',
      parameter: tronAbiEncodeAddressParam(account),
      visible: true
    }),
    tronPost(fetchImpl, baseUrl, '/wallet/triggerconstantcontract', {
      owner_address: account,
      contract_address: token,
      function_selector: 'decimals()',
      visible: true
    }).catch(() => null)
  ]);
  const balHex = bal?.constant_result?.[0] || '0';
  const raw = BigInt(`0x${stripHexPrefix(balHex) || '0'}`);
  const decHex = dec?.constant_result?.[0] || '0';
  const decimals = Number(BigInt(`0x${stripHexPrefix(decHex) || '0'}`) || 6n);
  return normalizeAmountResult({
    network: 'tron',
    account,
    token,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'tron triggerconstantcontract balanceOf'
  });
}
