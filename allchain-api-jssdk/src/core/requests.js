export function normalizeBalanceArgs(req = {}) {
  return {
    account: String(req.account || '').trim(),
    token: typeof req.token === 'string' ? req.token.trim() : ''
  };
}
