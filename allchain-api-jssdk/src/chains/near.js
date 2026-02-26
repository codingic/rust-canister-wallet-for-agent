import { decodeBytesToString, formatUnits, normalizeAmountResult, parseBigIntLoose, toBase64Utf8 } from '../utils/format.js';
import { jsonRpc } from '../utils/http.js';

async function nearRpc(fetchImpl, url, method, params) {
  return jsonRpc(fetchImpl, url, method, params);
}

async function nearCallFunction(fetchImpl, url, accountId, methodName, args) {
  const result = await nearRpc(fetchImpl, url, 'query', {
    request_type: 'call_function',
    finality: 'final',
    account_id: accountId,
    method_name: methodName,
    args_base64: toBase64Utf8(args)
  });
  const bytes = result?.result ?? [];
  const text = decodeBytesToString(bytes);
  return text ? JSON.parse(text) : null;
}

export async function nearGetBalance(fetchImpl, url, account) {
  try {
    const result = await nearRpc(fetchImpl, url, 'query', {
      request_type: 'view_account',
      finality: 'final',
      account_id: account
    });
    const yocto = parseBigIntLoose(result?.amount || '0');
    return normalizeAmountResult({
      network: 'near_mainnet',
      account,
      amount: formatUnits(yocto, 24),
      decimals: 24,
      message: 'near view_account'
    });
  } catch (err) {
    if (String(err.message || err).includes('UNKNOWN_ACCOUNT')) {
      return normalizeAmountResult({
        network: 'near_mainnet',
        account,
        amount: '0',
        decimals: 24,
        message: 'near unknown account -> 0'
      });
    }
    throw err;
  }
}

export async function nearGetNep141Balance(fetchImpl, url, account, tokenContract) {
  if (!tokenContract) throw new Error('token contract is required for NEP-141 balance');
  const [bal, meta] = await Promise.all([
    nearCallFunction(fetchImpl, url, tokenContract, 'ft_balance_of', { account_id: account }),
    nearCallFunction(fetchImpl, url, tokenContract, 'ft_metadata', {}).catch(() => null)
  ]);
  const raw = parseBigIntLoose(bal || '0');
  const decimals = Number(meta?.decimals ?? 0);
  return normalizeAmountResult({
    network: 'near_mainnet',
    account,
    token: tokenContract,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'near ft_balance_of'
  });
}
