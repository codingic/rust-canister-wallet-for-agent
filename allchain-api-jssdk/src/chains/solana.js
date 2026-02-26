import { formatUnits, normalizeAmountResult } from '../utils/format.js';
import { jsonRpc } from '../utils/http.js';

export async function solanaGetBalance(fetchImpl, rpcUrl, network, account) {
  const result = await jsonRpc(fetchImpl, rpcUrl, 'getBalance', [account, { commitment: 'confirmed' }]);
  const lamports = BigInt(result?.value || 0);
  return normalizeAmountResult({
    network,
    account,
    amount: formatUnits(lamports, 9),
    decimals: 9,
    message: 'solana getBalance'
  });
}

export async function solanaGetSplBalance(fetchImpl, rpcUrl, network, account, mint) {
  if (!mint) throw new Error('token (mint) is required for SPL balance');
  const tokenAccounts = await jsonRpc(fetchImpl, rpcUrl, 'getTokenAccountsByOwner', [
    account,
    { mint },
    { encoding: 'jsonParsed', commitment: 'confirmed' }
  ]);
  const rows = Array.isArray(tokenAccounts?.value) ? tokenAccounts.value : [];
  if (!rows.length) {
    return normalizeAmountResult({
      network,
      account,
      token: mint,
      amount: '0',
      decimals: 0,
      message: 'no token account for owner+mint'
    });
  }
  let totalRaw = 0n;
  let decimals = null;
  for (const row of rows) {
    const tokenAmount = row?.account?.data?.parsed?.info?.tokenAmount;
    if (!tokenAmount) continue;
    const amt = tokenAmount.amount ?? '0';
    totalRaw += BigInt(String(amt));
    if (decimals == null && Number.isFinite(Number(tokenAmount.decimals))) {
      decimals = Number(tokenAmount.decimals);
    }
  }
  if (decimals == null) {
    const supply = await jsonRpc(fetchImpl, rpcUrl, 'getTokenSupply', [mint, { commitment: 'confirmed' }]).catch(
      () => null
    );
    const d = supply?.value?.decimals;
    decimals = Number.isFinite(Number(d)) ? Number(d) : 0;
  }
  return normalizeAmountResult({
    network,
    account,
    token: mint,
    amount: formatUnits(totalRaw, decimals),
    decimals,
    message: 'solana getTokenAccountsByOwner'
  });
}
