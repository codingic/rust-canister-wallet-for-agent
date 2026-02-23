import { useEffect, useState } from 'react';

const NETWORK_CONFIG = {
  eth: {
    title: 'Ethereum',
    nativeSymbol: 'ETH',
    nativeLabel: 'ETH 地址',
    tokenLabel: 'Token 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  base: {
    title: 'Base',
    nativeSymbol: 'ETH',
    nativeLabel: 'Base 地址',
    tokenLabel: 'Token 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  bsc: {
    title: 'BNB Smart Chain',
    nativeSymbol: 'BNB',
    nativeLabel: 'BSC 地址',
    tokenLabel: 'BEP20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  arb: {
    title: 'Arbitrum',
    nativeSymbol: 'ETH',
    nativeLabel: 'Arbitrum 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  op: {
    title: 'Optimism',
    nativeSymbol: 'ETH',
    nativeLabel: 'Optimism 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  avax: {
    title: 'Avalanche',
    nativeSymbol: 'AVAX',
    nativeLabel: 'Avalanche 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC.e',
    showToken: true
  },
  okb: {
    title: 'OKB Chain',
    nativeSymbol: 'OKB',
    nativeLabel: 'OKB 链地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  polygon: {
    title: 'Polygon',
    nativeSymbol: 'POL',
    nativeLabel: 'Polygon 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  icp: {
    title: 'Internet Computer',
    nativeSymbol: 'ICP',
    nativeLabel: '账户地址',
    tokenLabel: 'ICRC Token Canister',
    tokenSymbol: 'ICRC',
    showToken: true
  },
  btc: {
    title: 'Bitcoin',
    nativeSymbol: 'BTC',
    nativeLabel: 'BTC 地址',
    tokenLabel: 'Token 地址',
    tokenSymbol: '',
    showToken: false
  },
  sol: {
    title: 'Solana',
    nativeSymbol: 'SOL',
    nativeLabel: 'Solana 地址',
    tokenLabel: 'SPL Token Mint',
    tokenSymbol: 'USDC',
    showToken: true
  },
  trx: {
    title: 'TRON',
    nativeSymbol: 'TRX',
    nativeLabel: 'TRX 地址',
    tokenLabel: 'TRC20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  ton: {
    title: 'TON',
    nativeSymbol: 'TON',
    nativeLabel: 'TON 地址',
    tokenLabel: 'Jetton Master 地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  near: {
    title: 'NEAR',
    nativeSymbol: 'NEAR',
    nativeLabel: 'NEAR 账户',
    tokenLabel: 'NEP-141 Token 合约',
    tokenSymbol: 'USDT',
    showToken: true
  },
  aptos: {
    title: 'Aptos',
    nativeSymbol: 'APT',
    nativeLabel: 'Aptos 地址',
    tokenLabel: 'Token 地址',
    tokenSymbol: 'APT',
    showToken: true
  },
  sui: {
    title: 'Sui',
    nativeSymbol: 'SUI',
    nativeLabel: 'Sui 地址',
    tokenLabel: 'Token Type',
    tokenSymbol: 'SUI',
    showToken: true
  }
};

const DEFAULT_NETWORK_ORDER = ['eth', 'base', 'bsc', 'arb', 'op', 'avax', 'okb', 'polygon', 'icp', 'btc', 'sol', 'trx', 'ton', 'near', 'aptos', 'sui'];

function fallbackNetworkConfig(networkId) {
  const upper = String(networkId || 'unknown').toUpperCase();
  return {
    title: upper,
    nativeSymbol: upper,
    nativeLabel: `${upper} 地址`,
    tokenLabel: 'Token 地址',
    tokenSymbol: 'Token',
    showToken: false
  };
}

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
    network: resp?.network || '',
    account: resp?.account || '',
    token: readOpt(resp?.token) || '',
    amount: readOpt(resp?.amount),
    decimals: readOpt(resp?.decimals),
    pending: Boolean(resp?.pending),
    blockRef: readOpt(resp?.block_ref),
    message: readOpt(resp?.message) || ''
  };
}

async function loadBackendActor() {
  const mod = await import('declarations/backend');
  return mod?.backend || null;
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

async function loadBackendSnapshot() {
  try {
    const actor = await loadBackendActor();
    if (!actor) {
      return {
        networks: null,
        serviceInfo: null,
        canisterId: null,
        source: 'missing-actor'
      };
    }
    const mod = await import('declarations/backend');

    const [networkRows, serviceInfoRaw] = await Promise.all([
      actor.supported_networks ? actor.supported_networks().catch(() => null) : Promise.resolve(null),
      actor.service_info ? actor.service_info().catch(() => null) : Promise.resolve(null)
    ]);

    const parsedRows =
      networkRows?.map((row) => ({
        network: typeof row?.network === 'string' ? row.network : '',
        balance_ready: Boolean(row?.balance_ready),
        transfer_ready: Boolean(row?.transfer_ready),
        note: readOpt(row?.note) || ''
      })) ?? null;

    const networks =
      parsedRows?.map((row) => row.network).filter((v) => typeof v === 'string' && v.trim().length > 0) ??
      null;

    return {
      networks: networks && networks.length ? [...new Set(networks)] : null,
      serviceInfo: parseServiceInfo(serviceInfoRaw),
      canisterId: typeof mod?.canisterId === 'string' ? mod.canisterId : null,
      source: 'backend'
    };
  } catch {
    return {
      networks: null,
      serviceInfo: null,
      canisterId: null,
      source: 'fallback'
    };
  }
}

function getBalanceMethodName(network, token = '') {
  const hasToken = typeof token === 'string' && token.trim().length > 0;
  if (network === 'eth') return hasToken ? 'eth_get_balance_erc20' : 'eth_get_balance_eth';
  if (network === 'base') return hasToken ? 'base_get_balance_erc20' : 'base_get_balance_eth';
  if (network === 'bsc') return hasToken ? 'bsc_get_balance_bep20' : 'bsc_get_balance_bnb';
  if (network === 'arb') return hasToken ? 'arb_get_balance_erc20' : 'arb_get_balance_eth';
  if (network === 'op') return hasToken ? 'op_get_balance_erc20' : 'op_get_balance_eth';
  if (network === 'avax') return hasToken ? 'avax_get_balance_erc20' : 'avax_get_balance_avax';
  if (network === 'okb') return hasToken ? 'okb_get_balance_erc20' : 'okb_get_balance_okb';
  if (network === 'polygon') return hasToken ? 'polygon_get_balance_erc20' : 'polygon_get_balance_pol';
  if (network === 'btc') return 'btc_get_balance_btc';
  if (network === 'icp') return hasToken ? 'icp_get_balance_icrc' : 'icp_get_balance_icp';
  if (network === 'sol') return hasToken ? 'sol_get_balance_spl' : 'sol_get_balance_sol';
  if (network === 'trx') return hasToken ? 'trx_get_balance_trc20' : 'trx_get_balance_trx';
  if (network === 'ton') return hasToken ? 'ton_get_balance_jetton' : 'ton_get_balance_ton';
  if (network === 'near') return hasToken ? 'near_get_balance_nep141' : 'near_get_balance_near';
  if (network === 'aptos') return hasToken ? 'aptos_get_balance_token' : 'aptos_get_balance_apt';
  if (network === 'sui') return hasToken ? 'sui_get_balance_token' : 'sui_get_balance_sui';
  return `${network}_get_balance`;
}

async function queryBalance(actor, network, account, token) {
  const methodName = getBalanceMethodName(network, token);
  const method = actor?.[methodName];
  if (typeof method !== 'function') {
    return { ok: false, error: `后端未暴露接口: ${methodName}` };
  }

  let result;
  try {
    result = await method({
      account,
      token: token ? [token] : []
    });
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : '调用后端失败'
    };
  }

  if (result?.Ok) {
    return { ok: true, data: parseBalanceResponse(result.Ok) };
  }
  if (result?.Err) {
    return { ok: false, error: formatWalletError(result.Err) };
  }
  return { ok: false, error: '后端返回格式不识别' };
}

export default function App() {
  const [networkOptions, setNetworkOptions] = useState(DEFAULT_NETWORK_ORDER);
  const [selectedNetwork, setSelectedNetwork] = useState(DEFAULT_NETWORK_ORDER[0]);
  const [nativeAddressInput, setNativeAddressInput] = useState('');
  const [tokenAddressInput, setTokenAddressInput] = useState('');
  const [statusText, setStatusText] = useState('初始化中...');
  const [toast, setToast] = useState(null);
  const [serviceInfo, setServiceInfo] = useState(null);
  const [backendCanisterId, setBackendCanisterId] = useState('');
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [nativeBalanceState, setNativeBalanceState] = useState({
    phase: 'idle',
    data: null,
    error: ''
  });
  const [tokenBalanceState, setTokenBalanceState] = useState({
    phase: 'idle',
    data: null,
    error: ''
  });

  const selectedConfig =
    NETWORK_CONFIG[selectedNetwork] || fallbackNetworkConfig(selectedNetwork);

  useEffect(() => {
    setNativeAddressInput('');
    setTokenAddressInput('');
    setNativeBalanceState({ phase: 'idle', data: null, error: '' });
    setTokenBalanceState({ phase: 'idle', data: null, error: '' });
    setStatusText(`已切换到 ${selectedConfig.title}，请输入地址后点击“刷新余额”`);
  }, [selectedNetwork, selectedConfig.title]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const snapshot = await loadBackendSnapshot();
      if (cancelled) return;

      if (snapshot.networks?.length) {
        setNetworkOptions(snapshot.networks);
        setSelectedNetwork((current) =>
          snapshot.networks.includes(current) ? current : snapshot.networks[0]
        );
      }
      if (snapshot.serviceInfo) {
        setServiceInfo(snapshot.serviceInfo);
      }
      if (snapshot.canisterId) {
        setBackendCanisterId(snapshot.canisterId);
      }

      if (snapshot.source === 'backend') {
        setStatusText('已连接后端：网络列表来自 canister `supported_networks()`');
      } else {
        setStatusText('未连接后端声明，使用本地网络配置（仍可预览 UI）');
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!toast) return undefined;
    const timer = window.setTimeout(() => setToast(null), 2200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  async function handleRefresh() {
    const address = nativeAddressInput.trim();
    const tokenAddress = tokenAddressInput.trim();
    if (!address) {
      const msg = `请先输入 ${selectedConfig.nativeLabel}`;
      setStatusText(msg);
      setToast(msg);
      return;
    }

    setIsRefreshing(true);
    setNativeBalanceState({ phase: 'loading', data: null, error: '' });
    if (selectedConfig.showToken) {
      if (tokenAddress) {
        setTokenBalanceState({ phase: 'loading', data: null, error: '' });
      } else {
        setTokenBalanceState({ phase: 'idle', data: null, error: '' });
      }
    } else {
      setTokenBalanceState({ phase: 'idle', data: null, error: '' });
    }

    setStatusText(`正在查询 ${selectedConfig.title} 余额...`);

    let actor = null;
    try {
      actor = await loadBackendActor();
    } catch {
      actor = null;
    }

    if (!actor) {
      setIsRefreshing(false);
      setNativeBalanceState({ phase: 'error', data: null, error: '前端未连接到 backend actor' });
      setStatusText('无法连接后端 actor，余额查询失败');
      setToast('无法连接后端 actor');
      return;
    }

    const snapshot = await loadBackendSnapshot();
    if (snapshot.networks?.length) {
      setNetworkOptions(snapshot.networks);
    }
    if (snapshot.serviceInfo) {
      setServiceInfo(snapshot.serviceInfo);
    }
    if (snapshot.canisterId) {
      setBackendCanisterId(snapshot.canisterId);
    }

    const nativeRes = await queryBalance(actor, selectedNetwork, address, '');
    if (nativeRes.ok) {
      setNativeBalanceState({ phase: 'ok', data: nativeRes.data, error: '' });
    } else {
      setNativeBalanceState({ phase: 'error', data: null, error: nativeRes.error });
    }

    if (selectedConfig.showToken && tokenAddress) {
      const tokenRes = await queryBalance(actor, selectedNetwork, address, tokenAddress);
      if (tokenRes.ok) {
        setTokenBalanceState({ phase: 'ok', data: tokenRes.data, error: '' });
      } else {
        setTokenBalanceState({ phase: 'error', data: null, error: tokenRes.error });
      }
    }

    setIsRefreshing(false);
    setStatusText(`已完成 ${selectedConfig.title} 查询（展示后端真实返回）`);
  }

  function handleLoginClick() {
    const msg = '登录逻辑待接入（可接 Internet Identity 或你自己的登录方案）';
    setStatusText(msg);
    setToast(msg);
  }

  const nativeBalanceValue =
    nativeBalanceState.phase === 'loading'
      ? '查询中...'
      : nativeBalanceState.phase === 'error'
        ? '查询失败'
        : nativeBalanceState.data?.amount || '未查询/无返回值';

  const nativeBalanceMeta =
    nativeBalanceState.phase === 'error'
      ? nativeBalanceState.error
      : nativeBalanceState.data?.message ||
        (nativeBalanceState.data?.pending ? '后端返回 pending=true' : '等待查询');

  const tokenBalanceValue =
    tokenBalanceState.phase === 'loading'
      ? '查询中...'
      : tokenBalanceState.phase === 'error'
        ? '查询失败'
        : tokenBalanceState.data?.amount || (selectedConfig.showToken ? '未查询/无返回值' : '--');

  const tokenBalanceMeta =
    tokenBalanceState.phase === 'error'
      ? tokenBalanceState.error
      : tokenBalanceState.data?.message ||
        (tokenBalanceState.data?.pending ? '后端返回 pending=true' : '输入 Token 地址后可查询');

  return (
    <div className="app-shell">
      <div className="bg-grid" aria-hidden="true" />
      <div className="bg-orb bg-orb--a" aria-hidden="true" />
      <div className="bg-orb bg-orb--b" aria-hidden="true" />
      {toast && (
        <div className="toast" role="status" aria-live="polite">
          <span className="toast__dot" aria-hidden="true" />
          <span>{toast}</span>
        </div>
      )}

      <header className="topbar">
        <div className="brand">
          <div className="brand__eyebrow">AGENT WALLET CONTROL PLANE</div>
          <div className="brand__title">rustwalletforagent</div>
          <div className="brand__meta" title={backendCanisterId || ''}>
            <span className="brand__meta-label">Backend Canister ID</span>
            <code className="brand__meta-value">{backendCanisterId || '未读取'}</code>
          </div>
        </div>

        <div className="topbar__actions">
          <label className="network-picker">
            <span className="network-picker__label">NETWORK</span>
            <select
              className="network-picker__select"
              value={selectedNetwork}
              onChange={(event) => setSelectedNetwork(event.target.value)}
              aria-label="选择网络"
            >
              {networkOptions.map((networkId) => {
                const cfg = NETWORK_CONFIG[networkId] || fallbackNetworkConfig(networkId);
                return (
                  <option key={networkId} value={networkId}>
                    {cfg.title}
                  </option>
                );
              })}
            </select>
          </label>

          <button type="button" className="button button--ghost" onClick={handleLoginClick}>
            登录
          </button>
        </div>
      </header>

      <main className="layout">
        <section className="hero panel">
          <div className="hero__scan" aria-hidden="true" />
          <div className="hero__content">
            <p className="hero__tag">NETWORK VIEW / LIVE PANEL</p>
            <h1 className="hero__title">{selectedConfig.title} 钱包面板</h1>
            <p className="hero__desc">
              选择网络后展示原生资产地址与余额，并显示 Token 地址与余额。
              当前不再使用模拟数据，点击“刷新余额”会直接调用后端 `{getBalanceMethodName(selectedNetwork)}`。
            </p>
          </div>
        </section>

        <section className="layout__main">
          <section className="panel control-panel" aria-labelledby="wallet-input-title">
            <div className="panel__header">
              <h2 id="wallet-input-title">钱包输入</h2>
              <p>后续接后端后，这里会驱动真实的余额查询与交易预填。</p>
            </div>

            <div className="field-grid">
              <label className="field">
                <span>{selectedConfig.nativeLabel}</span>
                <input
                  type="text"
                  value={nativeAddressInput}
                  onChange={(event) => setNativeAddressInput(event.target.value)}
                  placeholder="请输入地址"
                  autoComplete="off"
                />
              </label>

              {selectedConfig.showToken ? (
                <label className="field">
                  <span>{selectedConfig.tokenLabel}</span>
                  <input
                    type="text"
                    value={tokenAddressInput}
                    onChange={(event) => setTokenAddressInput(event.target.value)}
                    placeholder="请输入 Token 地址"
                    autoComplete="off"
                  />
                </label>
              ) : (
                <div className="field field--ghost" aria-hidden="true">
                  <span>Token</span>
                  <div className="field__placeholder">当前网络无 token 卡片展示</div>
                </div>
              )}
            </div>

            <div className="control-panel__actions">
              <button
                type="button"
                className="button button--primary"
                onClick={handleRefresh}
                disabled={isRefreshing}
              >
                {isRefreshing ? '查询中...' : '刷新余额'}
              </button>
              <p className="status-line" role="status">
                {statusText}
              </p>
            </div>
          </section>

          <section className="asset-grid" aria-label="资产卡片">
            <article className="panel asset-card asset-card--native">
              <header className="asset-card__head">
                <div>
                  <p className="asset-card__eyebrow">NATIVE ASSET</p>
                  <h2>{selectedConfig.nativeSymbol}</h2>
                </div>
                <span className="pill pill--glow">Primary</span>
              </header>

              <div className="asset-card__row">
                <div className="asset-card__label">地址</div>
                <div className="mono-block">{nativeAddressInput.trim() || '--'}</div>
              </div>

              <div className="asset-card__row">
                <div className="asset-card__label">余额</div>
                <div className="asset-card__balance">{nativeBalanceValue}</div>
                <div className="asset-card__sub">{nativeBalanceMeta}</div>
              </div>
            </article>

            {selectedConfig.showToken && (
              <article className="panel asset-card asset-card--token">
                <header className="asset-card__head">
                  <div>
                    <p className="asset-card__eyebrow">TOKEN ASSET</p>
                    <h2>{selectedConfig.tokenSymbol || 'Token'}</h2>
                  </div>
                  <span className="pill">Tracked</span>
                </header>

                <div className="asset-card__row">
                  <div className="asset-card__label">Token 地址</div>
                  <div className="mono-block">{tokenAddressInput.trim() || '未设置'}</div>
                </div>

                <div className="asset-card__row">
                  <div className="asset-card__label">余额</div>
                  <div className="asset-card__balance">{tokenBalanceValue}</div>
                  <div className="asset-card__sub">{tokenBalanceMeta}</div>
                </div>
              </article>
            )}
          </section>
        </section>

        <aside className="layout__side">
          <section className="panel side-panel side-panel--mono">
            <div className="panel__header">
              <h2>服务信息</h2>
              <p>来自 `service_info()` 的运行状态。</p>
            </div>
            <dl className="info-list">
              <div>
                <dt>Version</dt>
                <dd>{serviceInfo?.version || '--'}</dd>
              </div>
              <div>
                <dt>Owner</dt>
                <dd>{serviceInfo?.owner || '未读取'}</dd>
              </div>
              <div>
                <dt>Paused</dt>
                <dd>{serviceInfo ? String(serviceInfo.paused) : '--'}</dd>
              </div>
              <div>
                <dt>Note</dt>
                <dd>{serviceInfo?.note || '暂无'}</dd>
              </div>
            </dl>
          </section>
        </aside>
      </main>
    </div>
  );
}
