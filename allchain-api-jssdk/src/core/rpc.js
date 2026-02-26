import { BUILTIN_DEFAULT_RPCS } from '../config/networks.js';

export function createRpcResolver(overrides = {}) {
  const merged = { ...BUILTIN_DEFAULT_RPCS, ...(overrides || {}) };
  return (network) => {
    const url = merged[network];
    if (!url) throw new Error(`missing rpc for network: ${network}`);
    return url;
  };
}
