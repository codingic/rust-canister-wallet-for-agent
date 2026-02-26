export function ensureFetch(fetchImpl) {
  const f = fetchImpl || globalThis.fetch;
  if (typeof f !== 'function') {
    throw new Error('fetch is required (pass fetchImpl or use an environment with global fetch)');
  }
  return f;
}

export async function httpJson(fetchImpl, url, options = {}) {
  const f = ensureFetch(fetchImpl);
  const res = await f(url, options);
  const text = await res.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = null;
  }
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}${text ? `: ${text.slice(0, 240)}` : ''}`);
  }
  return parsed;
}

export async function jsonRpc(fetchImpl, url, method, params) {
  const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method, params });
  const parsed = await httpJson(fetchImpl, url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body
  });
  if (parsed?.error) {
    throw new Error(typeof parsed.error === 'string' ? parsed.error : JSON.stringify(parsed.error));
  }
  return parsed?.result;
}
