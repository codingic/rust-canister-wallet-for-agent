import { normalizeNetworkId } from '../config/networks';

function readOpt(value) {
  if (Array.isArray(value)) {
    return value.length > 0 ? value[0] : null;
  }
  return value ?? null;
}

function formatWalletError(err) {
  if (!err || typeof err !== 'object') return '未知错误';
  if ('InvalidInput' in err) return err.InvalidInput;
  if ('Internal' in err) return err.Internal;
  if ('Forbidden' in err) return '无权限';
  if ('Paused' in err) return '服务已暂停';
  if ('Unimplemented' in err) {
    const v = err.Unimplemented || {};
    return `未实现: ${v.network || 'unknown'}/${v.operation || 'unknown'}`;
  }
  return '未知错误';
}

function parseBalanceResponse(resp) {
  return {
    network: normalizeNetworkId(resp?.network || ''),
    account: resp?.account || '',
    token: readOpt(resp?.token) || '',
    amount: readOpt(resp?.amount),
    decimals: readOpt(resp?.decimals),
    pending: Boolean(resp?.pending),
    blockRef: readOpt(resp?.block_ref),
    message: readOpt(resp?.message) || ''
  };
}

function parseConfiguredToken(resp) {
  return {
    network: normalizeNetworkId(resp?.network || ''),
    symbol: resp?.symbol || '',
    name: resp?.name || '',
    tokenAddress: resp?.token_address || '',
    decimals: Number(resp?.decimals ?? 0)
  };
}

function parseConfiguredExplorer(resp) {
  return {
    network: normalizeNetworkId(resp?.network || ''),
    addressUrlTemplate: resp?.address_url_template || '',
    tokenUrlTemplate: readOpt(resp?.token_url_template) || ''
  };
}

function parseTransferResponse(resp) {
  const rawBroadcastReq = readOpt(resp?.broadcast_request);
  const broadcastRequest = rawBroadcastReq
    ? {
        url: rawBroadcastReq?.url || '',
        method: (rawBroadcastReq?.method || 'POST').toUpperCase(),
        headers: Array.isArray(rawBroadcastReq?.headers)
          ? rawBroadcastReq.headers
              .map((row) =>
                Array.isArray(row) && row.length >= 2
                  ? [String(row[0] || ''), String(row[1] || '')]
                  : null
              )
              .filter(Boolean)
          : [],
        body: readOpt(rawBroadcastReq?.body) || ''
      }
    : null;

  return {
    network: normalizeNetworkId(resp?.network || ''),
    accepted: Boolean(resp?.accepted),
    txId: readOpt(resp?.tx_id) || '',
    signedTx: readOpt(resp?.signed_tx) || '',
    signedTxEncoding: readOpt(resp?.signed_tx_encoding) || '',
    broadcastRequest,
    message: resp?.message || ''
  };
}

function parseAddressResponse(resp) {
  return {
    network: normalizeNetworkId(resp?.network || ''),
    address: resp?.address || '',
    publicKeyHex: resp?.public_key_hex || '',
    keyName: resp?.key_name || '',
    message: readOpt(resp?.message) || ''
  };
}

function parseServiceInfo(info) {
  if (!info) return null;
  const owner = readOpt(info.owner);
  const note = readOpt(info.note);

  return {
    version: info.version || '--',
    owner: owner?.toText?.() || String(owner || '未设置'),
    paused: Boolean(info.paused),
    caller: info.caller?.toText?.() || String(info.caller || '--'),
    note: note || ''
  };
}

function parseWalletNetworkInfo(row) {
  const id = normalizeNetworkId(typeof row?.id === 'string' ? row.id : '');
  return { id };
}

export async function loadBackendActor() {
  const mod = await import('declarations/backend');
  return mod?.backend || null;
}

export async function loadBackendSnapshot() {
  try {
    const actor = await loadBackendActor();
    if (!actor) {
      return {
        networks: null,
        networkNames: null,
        serviceInfo: null,
        canisterId: null,
        source: 'missing-actor'
      };
    }
    const mod = await import('declarations/backend');

    const [networkRows, walletNetworkRows, serviceInfoRaw] = await Promise.all([
      actor.supported_networks ? actor.supported_networks().catch(() => null) : Promise.resolve(null),
      actor.wallet_networks ? actor.wallet_networks().catch(() => null) : Promise.resolve(null),
      actor.service_info ? actor.service_info().catch(() => null) : Promise.resolve(null)
    ]);

    const parsedWalletRows =
      walletNetworkRows
        ?.map(parseWalletNetworkInfo)
        .filter((row) => typeof row.id === 'string' && row.id.length > 0) ??
      null;

    const parsedRows =
      networkRows?.map((row) => ({
        network: normalizeNetworkId(typeof row?.network === 'string' ? row.network : ''),
        balance_ready: Boolean(row?.balance_ready),
        transfer_ready: Boolean(row?.transfer_ready),
        note: readOpt(row?.note) || ''
      })) ?? null;

    const supportedNetworks =
      parsedRows
        ?.map((row) => row.network)
        .filter((v) => typeof v === 'string' && v.trim().length > 0) ??
      null;

    const walletNetworks = parsedWalletRows?.map((row) => row.id) ?? null;
    const networks = walletNetworks?.length ? walletNetworks : supportedNetworks;

    return {
      networks: networks && networks.length ? [...new Set(networks)] : null,
      networkNames: null,
      serviceInfo: parseServiceInfo(serviceInfoRaw),
      canisterId: typeof mod?.canisterId === 'string' ? mod.canisterId : null,
      source: 'backend'
    };
  } catch {
    return {
      networks: null,
      networkNames: null,
      serviceInfo: null,
      canisterId: null,
      source: 'fallback'
    };
  }
}

export async function queryConfiguredTokens(actor, network) {
  const method = actor?.configured_tokens;
  if (typeof method !== 'function') return [];
  try {
    const rows = await method(network);
    return Array.isArray(rows) ? rows.map(parseConfiguredToken) : [];
  } catch {
    return [];
  }
}

export async function queryConfiguredExplorer(actor, network) {
  const method = actor?.configured_explorer;
  if (typeof method !== 'function') return null;
  try {
    const opt = await method(network);
    const row = readOpt(opt);
    return row ? parseConfiguredExplorer(row) : null;
  } catch {
    return null;
  }
}

function getTransferMethodName(network, assetKind) {
  const n = normalizeNetworkId(network);
  const isToken = assetKind === 'token';
  if (n === 'ethereum') return isToken ? 'ethereum_transfer_erc20' : 'ethereum_transfer_eth';
  if (n === 'sepolia') return isToken ? 'sepolia_transfer_erc20' : 'sepolia_transfer_eth';
  if (n === 'base') return isToken ? 'base_transfer_erc20' : 'base_transfer_eth';
  if (n === 'bsc') return isToken ? 'bsc_transfer_bep20' : 'bsc_transfer_bnb';
  if (n === 'arbitrum') return isToken ? 'arbitrum_transfer_erc20' : 'arbitrum_transfer_eth';
  if (n === 'optimism') return isToken ? 'optimism_transfer_erc20' : 'optimism_transfer_eth';
  if (n === 'avalanche') return isToken ? 'avalanche_transfer_erc20' : 'avalanche_transfer_avax';
  if (n === 'okx') return isToken ? 'okx_transfer_erc20' : 'okx_transfer_okb';
  if (n === 'polygon') return isToken ? 'polygon_transfer_erc20' : 'polygon_transfer_pol';
  if (n === 'internet-computer') {
    return isToken ? 'internet_computer_transfer_icrc' : 'internet_computer_transfer_icp';
  }
  if (n === 'bitcoin') return 'bitcoin_transfer_btc';
  if (n === 'solana') return isToken ? 'solana_transfer_spl' : 'solana_transfer_sol';
  if (n === 'solana-testnet') {
    return isToken ? 'solana_testnet_transfer_spl' : 'solana_testnet_transfer_sol';
  }
  if (n === 'tron') return isToken ? 'tron_transfer_trc20' : 'tron_transfer_trx';
  if (n === 'ton-mainnet') return isToken ? 'ton_mainnet_transfer_jetton' : 'ton_mainnet_transfer_ton';
  if (n === 'near-mainnet') {
    return isToken ? 'near_mainnet_transfer_nep141' : 'near_mainnet_transfer_near';
  }
  if (n === 'aptos-mainnet') {
    return isToken ? 'aptos_mainnet_transfer_token' : 'aptos_mainnet_transfer_apt';
  }
  if (n === 'sui-mainnet') return isToken ? 'sui_mainnet_transfer_token' : 'sui_mainnet_transfer_sui';
  return `${n}_transfer`;
}

function tryParseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function readCommonBroadcastError(parsed) {
  if (!parsed || typeof parsed !== 'object') return '';
  if (parsed.error) return typeof parsed.error === 'string' ? parsed.error : JSON.stringify(parsed.error);
  if (parsed.result === false) {
    const msg = parsed.message || parsed.msg || parsed.error || parsed.code;
    return msg ? String(msg) : 'result=false';
  }
  if (parsed.ok === false) {
    return String(parsed.error || parsed.message || 'ok=false');
  }
  if (parsed.data && typeof parsed.data === 'object' && parsed.data.error) {
    return typeof parsed.data.error === 'string' ? parsed.data.error : JSON.stringify(parsed.data.error);
  }
  return '';
}

function extractBroadcastTxId(parsed) {
  if (!parsed || typeof parsed !== 'object') return '';
  return parsed.txid || parsed.txId || parsed.hash || parsed.digest || parsed.result || '';
}

async function broadcastPreparedTransfer(req) {
  if (!req?.url) return { ok: false, error: 'missing broadcast url' };
  const method = (req.method || 'POST').toUpperCase();
  const headers = {};
  for (const row of req.headers || []) {
    if (!Array.isArray(row) || row.length < 2) continue;
    const k = String(row[0] || '').trim();
    const v = String(row[1] || '');
    if (!k) continue;
    headers[k] = v;
  }

  let res;
  try {
    res = await fetch(req.url, {
      method,
      headers,
      body: req.body && method !== 'GET' ? req.body : undefined
    });
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : 'fetch broadcast failed'
    };
  }

  let text = '';
  try {
    text = await res.text();
  } catch {
    text = '';
  }

  const parsed = tryParseJson(text);
  const commonErr = readCommonBroadcastError(parsed);
  if (!res.ok) {
    return {
      ok: false,
      error: commonErr || `${res.status} ${res.statusText}${text ? `: ${text.slice(0, 240)}` : ''}`
    };
  }
  if (commonErr) {
    return { ok: false, error: commonErr };
  }

  const txIdRaw = extractBroadcastTxId(parsed);
  const txId = typeof txIdRaw === 'string' ? txIdRaw : '';
  return {
    ok: true,
    txId,
    message: txId ? `frontend broadcast ok tx=${txId}` : 'frontend broadcast ok'
  };
}

export function getAddressMethodName(network) {
  const n = normalizeNetworkId(network);
  if (n === 'ethereum') return 'ethereum_request_address';
  if (n === 'sepolia') return 'sepolia_request_address';
  if (n === 'base') return 'base_request_address';
  if (n === 'bsc') return 'bsc_request_address';
  if (n === 'arbitrum') return 'arbitrum_request_address';
  if (n === 'optimism') return 'optimism_request_address';
  if (n === 'avalanche') return 'avalanche_request_address';
  if (n === 'okx') return 'okx_request_address';
  if (n === 'polygon') return 'polygon_request_address';
  if (n === 'bitcoin') return 'bitcoin_request_address';
  if (n === 'solana') return 'solana_request_address';
  if (n === 'solana-testnet') return 'solana_testnet_request_address';
  if (n === 'tron') return 'tron_request_address';
  if (n === 'ton-mainnet') return 'ton_mainnet_request_address';
  if (n === 'near-mainnet') return 'near_mainnet_request_address';
  if (n === 'aptos-mainnet') return 'aptos_mainnet_request_address';
  if (n === 'sui-mainnet') return 'sui_mainnet_request_address';
  return null;
}

export async function queryRequestAddress(actor, network) {
  const methodName = getAddressMethodName(network);
  if (!methodName) return { ok: false, error: `后端未暴露地址申请接口: ${network}` };
  const method = actor?.[methodName];
  if (typeof method !== 'function') {
    return { ok: false, error: `后端未暴露接口: ${methodName}` };
  }

  let result;
  try {
    result = await method();
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : '调用地址申请接口失败'
    };
  }

  if (result?.Ok) return { ok: true, data: parseAddressResponse(result.Ok) };
  if (result?.Err) return { ok: false, error: formatWalletError(result.Err) };
  return { ok: false, error: '后端地址申请返回格式不识别' };
}

function getBalanceMethodName(network, token = '') {
  const n = normalizeNetworkId(network);
  const hasToken = typeof token === 'string' && token.trim().length > 0;
  if (n === 'ethereum') return hasToken ? 'ethereum_get_balance_erc20' : 'ethereum_get_balance_eth';
  if (n === 'sepolia') return hasToken ? 'sepolia_get_balance_erc20' : 'sepolia_get_balance_eth';
  if (n === 'base') return hasToken ? 'base_get_balance_erc20' : 'base_get_balance_eth';
  if (n === 'bsc') return hasToken ? 'bsc_get_balance_bep20' : 'bsc_get_balance_bnb';
  if (n === 'arbitrum') return hasToken ? 'arbitrum_get_balance_erc20' : 'arbitrum_get_balance_eth';
  if (n === 'optimism') return hasToken ? 'optimism_get_balance_erc20' : 'optimism_get_balance_eth';
  if (n === 'avalanche') return hasToken ? 'avalanche_get_balance_erc20' : 'avalanche_get_balance_avax';
  if (n === 'okx') return hasToken ? 'okx_get_balance_erc20' : 'okx_get_balance_okb';
  if (n === 'polygon') return hasToken ? 'polygon_get_balance_erc20' : 'polygon_get_balance_pol';
  if (n === 'bitcoin') return 'bitcoin_get_balance_btc';
  if (n === 'internet-computer') {
    return hasToken ? 'internet_computer_get_balance_icrc' : 'internet_computer_get_balance_icp';
  }
  if (n === 'solana') return hasToken ? 'solana_get_balance_spl' : 'solana_get_balance_sol';
  if (n === 'solana-testnet') {
    return hasToken ? 'solana_testnet_get_balance_spl' : 'solana_testnet_get_balance_sol';
  }
  if (n === 'tron') return hasToken ? 'tron_get_balance_trc20' : 'tron_get_balance_trx';
  if (n === 'ton-mainnet') return hasToken ? 'ton_mainnet_get_balance_jetton' : 'ton_mainnet_get_balance_ton';
  if (n === 'near-mainnet') {
    return hasToken ? 'near_mainnet_get_balance_nep141' : 'near_mainnet_get_balance_near';
  }
  if (n === 'aptos-mainnet') {
    return hasToken ? 'aptos_mainnet_get_balance_token' : 'aptos_mainnet_get_balance_apt';
  }
  if (n === 'sui-mainnet') return hasToken ? 'sui_mainnet_get_balance_token' : 'sui_mainnet_get_balance_sui';
  return `${n}_get_balance`;
}

export async function queryBalance(actor, network, account, token) {
  const methodName = getBalanceMethodName(network, token);
  const method = actor?.[methodName];
  if (typeof method !== 'function') {
    return { ok: false, error: `后端未暴露接口: ${methodName}` };
  }

  let result;
  try {
    result = await method({ account, token: token ? [token] : [] });
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : '调用后端失败'
    };
  }

  if (result?.Ok) return { ok: true, data: parseBalanceResponse(result.Ok) };
  if (result?.Err) return { ok: false, error: formatWalletError(result.Err) };
  return { ok: false, error: '后端返回格式不识别' };
}

export async function queryTransfer(actor, network, asset, fromAddress, toAddress, amount) {
  const methodName = getTransferMethodName(network, asset?.kind || 'token');
  const method = actor?.[methodName];
  if (typeof method !== 'function') {
    return { ok: false, error: `后端未暴露接口: ${methodName}` };
  }

  const tokenAddress = asset?.kind === 'token' ? String(asset?.tokenAddress || '').trim() : '';
  let result;
  try {
    result = await method({
      from: fromAddress ? [fromAddress] : [],
      to: toAddress,
      amount,
      token: tokenAddress ? [tokenAddress] : [],
      memo: [],
      nonce: [],
      metadata: []
    });
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : '调用后端发送接口失败'
    };
  }

  if (result?.Ok) {
    const data = parseTransferResponse(result.Ok);
    if (data.broadcastRequest?.url) {
      const broadcasted = await broadcastPreparedTransfer(data.broadcastRequest);
      if (!broadcasted.ok) {
        return {
          ok: false,
          error: `前端广播失败: ${broadcasted.error}`,
          prepared: data
        };
      }
      data.accepted = true;
      const suffix = broadcasted.message ? `; ${broadcasted.message}` : '';
      data.message = `${data.message}${suffix}`;
      if (!data.txId && broadcasted.txId) {
        data.txId = broadcasted.txId;
      }
    }
    return { ok: true, data };
  }
  if (result?.Err) return { ok: false, error: formatWalletError(result.Err) };
  return { ok: false, error: '后端发送接口返回格式不识别' };
}

function fillExplorerTemplate(template, params) {
  if (!template) return '';
  return String(template)
    .replaceAll('{address}', encodeURIComponent(params.address || ''))
    .replaceAll('{token}', encodeURIComponent(params.token || ''));
}

export function buildExplorerUrlFromConfig(config, account, tokenAddress) {
  const address = String(account || '').trim();
  const token = String(tokenAddress || '').trim();
  if (!config || !address) return '';

  if (token && config.tokenUrlTemplate) {
    return fillExplorerTemplate(config.tokenUrlTemplate, { address, token });
  }
  return fillExplorerTemplate(config.addressUrlTemplate, { address, token });
}
