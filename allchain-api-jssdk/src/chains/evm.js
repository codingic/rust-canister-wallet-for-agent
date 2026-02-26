import { formatUnits, hexToBigInt, normalizeAmountResult, stripHexPrefix } from '../utils/format.js';
import { jsonRpc } from '../utils/http.js';

function pad32Hex(hexNoPrefix) {
  return hexNoPrefix.padStart(64, '0');
}

function evmAddressTo32(address) {
  const s = stripHexPrefix(address).toLowerCase();
  if (!/^[0-9a-f]{40}$/.test(s)) throw new Error('invalid EVM address');
  return pad32Hex(s);
}

async function evmCall(fetchImpl, url, to, data) {
  return jsonRpc(fetchImpl, url, 'eth_call', [{ to, data }, 'latest']);
}

export async function evmGetNativeBalance(fetchImpl, rpcUrl, network, account, nativeDecimals = 18) {
  const result = await jsonRpc(fetchImpl, rpcUrl, 'eth_getBalance', [account, 'latest']);
  const wei = hexToBigInt(result || '0x0');
  return normalizeAmountResult({
    network,
    account,
    amount: formatUnits(wei, nativeDecimals),
    decimals: nativeDecimals,
    message: 'rpc eth_getBalance'
  });
}

export async function evmGetTokenBalance(fetchImpl, rpcUrl, network, account, tokenAddress) {
  if (!tokenAddress) throw new Error('token is required for token balance');
  const balanceData = `0x70a08231${evmAddressTo32(account)}`;
  const [balHex, decHex] = await Promise.all([
    evmCall(fetchImpl, rpcUrl, tokenAddress, balanceData),
    evmCall(fetchImpl, rpcUrl, tokenAddress, '0x313ce567').catch(() => '0x')
  ]);
  const raw = hexToBigInt(balHex || '0x0');
  const decimals = decHex && stripHexPrefix(decHex).length > 0 ? Number(hexToBigInt(decHex)) : 18;
  return normalizeAmountResult({
    network,
    account,
    token: tokenAddress,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'rpc eth_call balanceOf'
  });
}
