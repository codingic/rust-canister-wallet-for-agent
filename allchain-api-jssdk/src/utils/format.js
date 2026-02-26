const TEXT_ENCODER = new TextEncoder();

export function toBase64Utf8(value) {
  const text = typeof value === 'string' ? value : JSON.stringify(value);
  if (typeof Buffer !== 'undefined') return Buffer.from(text, 'utf8').toString('base64');
  let binary = '';
  for (const byte of TEXT_ENCODER.encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary);
}

export function decodeBytesToString(bytesLike) {
  if (typeof bytesLike === 'string') return bytesLike;
  const arr = Array.isArray(bytesLike) ? Uint8Array.from(bytesLike) : new Uint8Array();
  return new TextDecoder().decode(arr);
}

export function stripHexPrefix(v) {
  const s = String(v || '');
  return s.startsWith('0x') || s.startsWith('0X') ? s.slice(2) : s;
}

export function hexToBigInt(v) {
  const s = stripHexPrefix(v || '0');
  return BigInt(`0x${s || '0'}`);
}

export function parseBigIntLoose(value) {
  if (typeof value === 'bigint') return value;
  if (typeof value === 'number') return BigInt(value);
  if (typeof value === 'string') {
    const s = value.trim();
    if (!s) return 0n;
    if (s.startsWith('0x') || s.startsWith('0X')) return BigInt(s);
    return BigInt(s);
  }
  if (value && typeof value === 'object' && typeof value.toString === 'function') {
    return BigInt(value.toString());
  }
  return 0n;
}

export function formatUnits(value, decimals = 0) {
  const n = parseBigIntLoose(value);
  const d = Number(decimals || 0);
  if (!Number.isFinite(d) || d <= 0) return n.toString();
  const sign = n < 0n ? '-' : '';
  const abs = n < 0n ? -n : n;
  const base = 10n ** BigInt(d);
  const whole = abs / base;
  const frac = (abs % base).toString().padStart(d, '0').replace(/0+$/, '');
  return frac ? `${sign}${whole}.${frac}` : `${sign}${whole}`;
}

export function normalizeAmountResult({ network, account, token = '', amount, decimals, message = '' }) {
  return {
    network,
    account,
    token: token || '',
    amount: amount == null ? null : String(amount),
    decimals: decimals == null ? null : Number(decimals),
    pending: false,
    blockRef: null,
    message
  };
}
