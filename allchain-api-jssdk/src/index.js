import { BALANCE_METHOD_PATTERN, BUILTIN_DEFAULT_RPCS } from './config/networks.js';
import { makeMethodTable } from './core/methodTable.js';
import { createRpcResolver } from './core/rpc.js';
import { ensureFetch } from './utils/http.js';

export function createAllChainApiClient(options = {}) {
  const fetchImpl = ensureFetch(options.fetchImpl);
  const rpc = createRpcResolver(options.rpc);
  const ctx = {
    fetch: fetchImpl,
    rpc,
    icp: options.icp || {}
  };

  const methods = makeMethodTable(ctx);

  return {
    ...methods,
    rpc,
    listBalanceMethods() {
      return Object.keys(methods).sort();
    },
    async getBalanceByMethod(methodName, req) {
      const fn = methods[methodName];
      if (typeof fn !== 'function') throw new Error(`unsupported balance method: ${methodName}`);
      return fn(req);
    }
  };
}

export { BALANCE_METHOD_PATTERN };
export const DEFAULT_RPCS = { ...BUILTIN_DEFAULT_RPCS };
