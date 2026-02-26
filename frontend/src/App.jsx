import { useEffect, useState } from 'react';
import AssetDetailModal from './components/AssetDetailModal';
import AssetPanel from './components/AssetPanel';
import TopBar from './components/TopBar';
import {
  DEFAULT_NETWORK_ORDER,
  NETWORK_CONFIG,
  TOKEN_VLIST_HEIGHT,
  TOKEN_VLIST_OVERSCAN,
  TOKEN_VLIST_ROW_HEIGHT,
  fallbackNetworkConfig
} from './config/networks';
import {
  buildExplorerUrlFromConfig,
  getAddressMethodName,
  loadBackendActor,
  loadBackendSnapshot,
  queryBalance,
  queryConfiguredExplorer,
  queryConfiguredTokens,
  queryRequestAddress,
  queryTransfer
} from './api/backend';

export default function App() {
  const [networkOptions, setNetworkOptions] = useState(DEFAULT_NETWORK_ORDER);
  const [networkDisplayNames, setNetworkDisplayNames] = useState({});
  const [lang, setLang] = useState(() => {
    try {
      const saved = window.localStorage.getItem('app_lang');
      return saved === 'en' ? 'en' : 'zh';
    } catch {
      return 'zh';
    }
  });
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
  const [configuredTokens, setConfiguredTokens] = useState([]);
  const [configuredExplorer, setConfiguredExplorer] = useState(null);
  const [selectedConfiguredTokenAddress, setSelectedConfiguredTokenAddress] = useState('');
  const [tokenListScrollTop, setTokenListScrollTop] = useState(0);
  const [tokenRowBalances, setTokenRowBalances] = useState({});
  const [selectedAssetRowKey, setSelectedAssetRowKey] = useState('__native__');
  const [tokenDetailAddress, setTokenDetailAddress] = useState('');
  const [tokenTransferTo, setTokenTransferTo] = useState('');
  const [tokenTransferAmount, setTokenTransferAmount] = useState('');
  const [isTokenSending, setIsTokenSending] = useState(false);
  const [detailBalanceState, setDetailBalanceState] = useState({
    phase: 'idle',
    data: null,
    error: ''
  });

  const selectedConfig =
    NETWORK_CONFIG[selectedNetwork] || fallbackNetworkConfig(selectedNetwork);
  const isZh = lang === 'zh';
  const trText = (zh, en) => (isZh ? zh : en);

  useEffect(() => {
    try {
      window.localStorage.setItem('app_lang', lang);
    } catch {
      // ignore storage errors
    }
  }, [lang]);

  useEffect(() => {
    setNativeAddressInput('');
    setTokenAddressInput('');
    setConfiguredTokens([]);
    setConfiguredExplorer(null);
    setSelectedConfiguredTokenAddress('');
    setTokenListScrollTop(0);
    setTokenRowBalances({});
    setSelectedAssetRowKey('__native__');
    setTokenDetailAddress('');
    setTokenTransferTo('');
    setTokenTransferAmount('');
    setDetailBalanceState({ phase: 'idle', data: null, error: '' });
    setNativeBalanceState({ phase: 'idle', data: null, error: '' });
    setTokenBalanceState({ phase: 'idle', data: null, error: '' });
    setStatusText(
      trText(`已切换到 ${selectedConfig.title}`, `Switched to ${selectedConfig.title}`)
    );
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
      if (snapshot.networkNames && Object.keys(snapshot.networkNames).length > 0) {
        setNetworkDisplayNames(snapshot.networkNames);
      }
      if (snapshot.serviceInfo) {
        setServiceInfo(snapshot.serviceInfo);
      }
      if (snapshot.canisterId) {
        setBackendCanisterId(snapshot.canisterId);
      }

      if (snapshot.source === 'backend') {
        setStatusText('已连接后端：网络列表与名称来自 canister 接口');
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

  useEffect(() => {
    let cancelled = false;

    (async () => {
      if (!selectedConfig.showToken) {
        setConfiguredTokens([]);
        setSelectedConfiguredTokenAddress('');
        setTokenAddressInput('');
        return;
      }

      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }

      if (!actor) {
        if (!cancelled) {
          setConfiguredTokens([]);
          setSelectedConfiguredTokenAddress('');
          setTokenAddressInput('');
        }
        return;
      }

      const tokens = await queryConfiguredTokens(actor, selectedNetwork);
      if (cancelled) return;

      setConfiguredTokens(tokens);
      const firstAddr = tokens[0]?.tokenAddress || '';
      setSelectedConfiguredTokenAddress(firstAddr);
      setTokenAddressInput(firstAddr);
    })();

    return () => {
      cancelled = true;
    };
  }, [selectedNetwork, selectedConfig.showToken]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }
      if (!actor) {
        if (!cancelled) setConfiguredExplorer(null);
        return;
      }

      const explorer = await queryConfiguredExplorer(actor, selectedNetwork);
      if (cancelled) return;
      setConfiguredExplorer(explorer);
    })();

    return () => {
      cancelled = true;
    };
  }, [selectedNetwork]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const addressMethod = getAddressMethodName(selectedNetwork);
      const canUseIcpCanisterId = selectedNetwork === 'internet-computer';
      if (!addressMethod && !canUseIcpCanisterId) return;

      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }
      if (!actor || cancelled) return;

      setNativeBalanceState({ phase: 'loading', data: null, error: '' });
      setStatusText(`正在申请 ${selectedConfig.title} 地址并查询 ${selectedConfig.nativeSymbol} 余额...`);

      let autoAddress = '';
      if (canUseIcpCanisterId) {
        autoAddress = (backendCanisterId || '').trim();
        if (!autoAddress) {
          setNativeAddressInput('');
          setNativeBalanceState({
            phase: 'error',
            data: null,
            error: '未读取到 backend canister id'
          });
          setStatusText('ICP 地址加载失败: 未读取到 backend canister id');
          return;
        }
      } else {
        const addrRes = await queryRequestAddress(actor, selectedNetwork);
        if (cancelled) return;
        if (!addrRes.ok) {
          setNativeAddressInput('');
          setNativeBalanceState({ phase: 'error', data: null, error: addrRes.error });
          setStatusText(`${selectedConfig.title} 地址申请失败: ${addrRes.error}`);
          return;
        }
        autoAddress = addrRes.data.address;
      }
      setNativeAddressInput(autoAddress);

      const balRes = await queryBalance(actor, selectedNetwork, autoAddress, '');
      if (cancelled) return;
      if (balRes.ok) {
        setNativeBalanceState({ phase: 'ok', data: balRes.data, error: '' });
        setStatusText(`已自动加载 ${selectedConfig.title} 地址与 ${selectedConfig.nativeSymbol} 余额`);
      } else {
        setNativeBalanceState({ phase: 'error', data: null, error: balRes.error });
        setStatusText(`${selectedConfig.nativeSymbol} 余额查询失败: ${balRes.error}`);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [backendCanisterId, selectedConfig.nativeSymbol, selectedConfig.title, selectedNetwork]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      if (!selectedConfig.showToken) {
        setTokenBalanceState({ phase: 'idle', data: null, error: '' });
        return;
      }

      const address = nativeAddressInput.trim();
      const tokenAddress = tokenAddressInput.trim();
      if (!address || !tokenAddress) {
        setTokenBalanceState({ phase: 'idle', data: null, error: '' });
        return;
      }

      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }
      if (!actor || cancelled) {
        if (!cancelled) {
          setTokenBalanceState({ phase: 'error', data: null, error: '前端未连接到 backend actor' });
        }
        return;
      }

      setTokenBalanceState({ phase: 'loading', data: null, error: '' });
      const tokenRes = await queryBalance(actor, selectedNetwork, address, tokenAddress);
      if (cancelled) return;
      if (tokenRes.ok) {
        setTokenBalanceState({ phase: 'ok', data: tokenRes.data, error: '' });
      } else {
        setTokenBalanceState({ phase: 'error', data: null, error: tokenRes.error });
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [selectedConfig.showToken, selectedNetwork, nativeAddressInput, tokenAddressInput]);

  useEffect(() => {
    setTokenRowBalances({});
  }, [selectedNetwork, nativeAddressInput]);

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
    const msg = trText(
      '登录逻辑待接入（可接 Internet Identity 或你自己的登录方案）',
      'Login flow is not wired yet (Internet Identity or your own auth can be integrated).'
    );
    setStatusText(msg);
    setToast(msg);
  }

  const nativeBalanceValue =
    nativeBalanceState.phase === 'loading'
      ? trText('查询中...', 'Loading...')
      : nativeBalanceState.phase === 'error'
        ? trText('查询失败', 'Query failed')
        : nativeBalanceState.data?.amount || trText('未查询/无返回值', 'No result');

  const nativeBalanceMeta =
    nativeBalanceState.phase === 'error'
      ? nativeBalanceState.error
      : nativeBalanceState.data?.message ||
        (nativeBalanceState.data?.pending
          ? trText('后端返回 pending=true', 'Backend returned pending=true')
          : trText('等待查询', 'Waiting for query'));

  const tokenBalanceValue =
    tokenBalanceState.phase === 'loading'
      ? trText('查询中...', 'Loading...')
      : tokenBalanceState.phase === 'error'
        ? trText('查询失败', 'Query failed')
        : tokenBalanceState.data?.amount ||
          (selectedConfig.showToken ? trText('未查询/无返回值', 'No result') : '--');

  const tokenBalanceMeta =
    tokenBalanceState.phase === 'error'
      ? tokenBalanceState.error
      : tokenBalanceState.data?.message ||
        (tokenBalanceState.data?.pending
          ? trText('后端返回 pending=true', 'Backend returned pending=true')
          : configuredTokens.length
            ? trText(
                `已从 config 加载 ${configuredTokens.length} 个 Token`,
                `Loaded ${configuredTokens.length} token(s) from config`
              )
            : trText('当前网络 config 未配置 Token', 'No tokens configured for this network'));

  const selectedConfiguredToken =
    configuredTokens.find((t) => t.tokenAddress === selectedConfiguredTokenAddress) ||
    configuredTokens[0] ||
    null;
  const nativeDecimals = nativeBalanceState.data?.decimals ?? null;
  const nativeAssetRow = {
    rowKey: '__native__',
    kind: 'native',
    symbol: selectedConfig.nativeSymbol,
    name: `${selectedConfig.title} Native`,
    tokenAddress: '',
    decimals: nativeDecimals,
    network: selectedNetwork
  };
  const assetListItems = [
    nativeAssetRow,
    ...configuredTokens.map((token) => ({
      rowKey: `token:${token.tokenAddress}`,
      kind: 'token',
      symbol: token.symbol,
      name: token.name,
      tokenAddress: token.tokenAddress,
      decimals: token.decimals,
      network: token.network
    }))
  ];
  const selectedAsset =
    assetListItems.find((item) => item.rowKey === selectedAssetRowKey) || assetListItems[0] || null;
  const detailAsset =
    assetListItems.find((item) =>
      item.kind === 'native' ? tokenDetailAddress === '__native__' : item.tokenAddress === tokenDetailAddress
    ) || null;
  const detailTokenRowBalance =
    detailAsset && detailAsset.kind === 'token'
      ? tokenRowBalances[detailAsset.tokenAddress] || null
      : null;
  const detailTokenBalanceValue =
    detailTokenRowBalance?.phase === 'loading'
      ? trText('查询中...', 'Loading...')
      : detailTokenRowBalance?.phase === 'error'
        ? trText('查询失败', 'Query failed')
        : detailTokenRowBalance?.amount || tokenBalanceValue;
  const detailTokenBalanceMeta =
    detailTokenRowBalance?.phase === 'error'
      ? detailTokenRowBalance.error
      : detailTokenRowBalance?.message ||
        (detailTokenRowBalance?.pending ? 'pending=true' : tokenBalanceMeta);
  const detailBalanceValue =
    detailBalanceState.phase === 'loading'
      ? trText('查询中...', 'Loading...')
      : detailBalanceState.phase === 'error'
        ? trText('查询失败', 'Query failed')
        : detailBalanceState.data?.amount ||
          (detailAsset?.kind === 'native' ? nativeBalanceValue : detailTokenBalanceValue);
  const detailBalanceMeta =
    detailBalanceState.phase === 'error'
      ? detailBalanceState.error
      : detailBalanceState.data?.message ||
        (detailBalanceState.data?.pending
          ? trText('后端返回 pending=true', 'Backend returned pending=true')
          : detailAsset?.kind === 'native'
            ? nativeBalanceMeta
            : detailTokenBalanceMeta);

  const tokenListCount = assetListItems.length;
  const tokenVisibleRows = Math.max(1, Math.ceil(TOKEN_VLIST_HEIGHT / TOKEN_VLIST_ROW_HEIGHT));
  const tokenStartIndex = Math.max(
    0,
    Math.floor(tokenListScrollTop / TOKEN_VLIST_ROW_HEIGHT) - TOKEN_VLIST_OVERSCAN
  );
  const tokenEndIndex = Math.min(
    tokenListCount,
    tokenStartIndex + tokenVisibleRows + TOKEN_VLIST_OVERSCAN * 2
  );
  const visibleAssetItems = assetListItems.slice(tokenStartIndex, tokenEndIndex);
  const visibleTokenItems = visibleAssetItems.filter((item) => item.kind === 'token');
  const visibleTokenAddressesKey = visibleTokenItems.map((t) => t.tokenAddress).join('|');

  useEffect(() => {
    let cancelled = false;

    (async () => {
      if (!selectedConfig.showToken) return;
      const account = nativeAddressInput.trim();
      if (!account || !visibleTokenItems.length) return;

      const pendingTokens = visibleTokenItems.filter((token) => {
        const state = tokenRowBalances[token.tokenAddress];
        return !state || state.phase === 'idle';
      });
      if (!pendingTokens.length) return;

      setTokenRowBalances((prev) => {
        const next = { ...prev };
        for (const token of pendingTokens) {
          next[token.tokenAddress] = { phase: 'loading', amount: '', error: '' };
        }
        return next;
      });

      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }
      if (!actor || cancelled) {
        if (!cancelled) {
          setTokenRowBalances((prev) => {
            const next = { ...prev };
            for (const token of pendingTokens) {
              next[token.tokenAddress] = {
                phase: 'error',
                amount: '',
                error: '前端未连接到 backend actor'
              };
            }
            return next;
          });
        }
        return;
      }

      const results = await Promise.all(
        pendingTokens.map(async (token) => {
          const resp = await queryBalance(actor, selectedNetwork, account, token.tokenAddress);
          return { tokenAddress: token.tokenAddress, resp };
        })
      );
      if (cancelled) return;

      setTokenRowBalances((prev) => {
        const next = { ...prev };
        for (const row of results) {
          if (row.resp.ok) {
            next[row.tokenAddress] = {
              phase: 'ok',
              amount: row.resp.data?.amount || '',
              error: '',
              pending: Boolean(row.resp.data?.pending),
              message: row.resp.data?.message || ''
            };
          } else {
            next[row.tokenAddress] = {
              phase: 'error',
              amount: '',
              error: row.resp.error || '查询失败'
            };
          }
        }
        return next;
      });
    })();

    return () => {
      cancelled = true;
    };
  }, [selectedConfig.showToken, selectedNetwork, nativeAddressInput, visibleTokenAddressesKey]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      if (!detailAsset) {
        setDetailBalanceState({ phase: 'idle', data: null, error: '' });
        return;
      }

      const account = nativeAddressInput.trim();
      if (!account) {
        setDetailBalanceState({ phase: 'error', data: null, error: '当前钱包地址未就绪' });
        return;
      }

      let actor = null;
      try {
        actor = await loadBackendActor();
      } catch {
        actor = null;
      }
      if (!actor || cancelled) {
        if (!cancelled) {
          setDetailBalanceState({
            phase: 'error',
            data: null,
            error: '前端未连接到 backend actor'
          });
        }
        return;
      }

      setDetailBalanceState({ phase: 'loading', data: null, error: '' });
      const token =
        detailAsset.kind === 'token' ? String(detailAsset.tokenAddress || '').trim() : '';
      const result = await queryBalance(actor, selectedNetwork, account, token);
      if (cancelled) return;

      if (result.ok) {
        setDetailBalanceState({ phase: 'ok', data: result.data, error: '' });

        if (detailAsset.kind === 'native') {
          setNativeBalanceState({ phase: 'ok', data: result.data, error: '' });
        } else if (token) {
          setTokenBalanceState({ phase: 'ok', data: result.data, error: '' });
          setTokenRowBalances((prev) => ({
            ...prev,
            [token]: {
              phase: 'ok',
              amount: result.data?.amount || '',
              error: '',
              pending: Boolean(result.data?.pending),
              message: result.data?.message || ''
            }
          }));
        }
      } else {
        setDetailBalanceState({ phase: 'error', data: null, error: result.error });
        if (detailAsset.kind === 'token' && token) {
          setTokenRowBalances((prev) => ({
            ...prev,
            [token]: { phase: 'error', amount: '', error: result.error || '查询失败' }
          }));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [detailAsset?.kind, detailAsset?.tokenAddress, nativeAddressInput, selectedNetwork]);

  function openTokenDetail(asset) {
    if (!asset) return;
    setSelectedAssetRowKey(asset.rowKey);
    if (asset.kind === 'native') {
      setTokenAddressInput('');
      setTokenDetailAddress('__native__');
      setTokenBalanceState({ phase: 'idle', data: null, error: '' });
    } else {
      const nextAddress = asset.tokenAddress || '';
      setSelectedConfiguredTokenAddress(nextAddress);
      setTokenAddressInput(nextAddress);
      setTokenDetailAddress(nextAddress);
      setTokenBalanceState({ phase: 'idle', data: null, error: '' });
    }
    setTokenTransferTo('');
    setTokenTransferAmount('');
    setDetailBalanceState({ phase: 'idle', data: null, error: '' });
  }

  function closeTokenDetail() {
    setTokenDetailAddress('');
    setTokenTransferTo('');
    setTokenTransferAmount('');
    setDetailBalanceState({ phase: 'idle', data: null, error: '' });
  }

  async function handleTokenSendClick() {
    const asset = detailAsset;
    if (!asset) {
      const msg = trText('请先选择资产', 'Please select an asset first');
      setToast(msg);
      setStatusText(msg);
      return;
    }
    if (!nativeAddressInput.trim()) {
      const msg = trText(
        '当前钱包地址未就绪，无法发送',
        'Wallet address is not ready; cannot send'
      );
      setToast(msg);
      setStatusText(msg);
      return;
    }
    if (!tokenTransferTo.trim()) {
      const msg = trText('请输入 To 地址', 'Please enter a destination address');
      setToast(msg);
      setStatusText(msg);
      return;
    }
    if (!tokenTransferAmount.trim()) {
      const msg = trText('请输入发送数量', 'Please enter an amount to send');
      setToast(msg);
      setStatusText(msg);
      return;
    }

    let actor = null;
    try {
      actor = await loadBackendActor();
    } catch {
      actor = null;
    }
    if (!actor) {
      const msg = trText('前端未连接到 backend actor', 'Frontend is not connected to backend actor');
      setToast(msg);
      setStatusText(msg);
      return;
    }

    setIsTokenSending(true);
    setStatusText(
      trText(
        `正在发送 ${selectedNetwork} ${asset.symbol || 'Asset'} ...`,
        `Sending ${selectedNetwork} ${asset.symbol || 'Asset'} ...`
      )
    );

    const sendRes = await queryTransfer(
      actor,
      selectedNetwork,
      asset,
      nativeAddressInput.trim(),
      tokenTransferTo.trim(),
      tokenTransferAmount.trim()
    );

    if (sendRes.ok) {
      const txLabel = sendRes.data?.txId ? ` tx=${sendRes.data.txId}` : '';
      const msg = sendRes.data?.accepted
        ? trText(`发送成功${txLabel}`, `Sent successfully${txLabel}`)
        : trText(
            `发送未执行: ${sendRes.data?.message || '后端返回 accepted=false'}`,
            `Send not executed: ${sendRes.data?.message || 'backend returned accepted=false'}`
          );
      setStatusText(msg);
      setToast(msg);
    } else {
      const msg = trText(`发送失败: ${sendRes.error}`, `Send failed: ${sendRes.error}`);
      setStatusText(msg);
      setToast(msg);
    }

    setIsTokenSending(false);
  }

  function handleOpenExplorerClick() {
    if (!detailAsset) {
      const msg = trText('当前未选中资产', 'No asset selected');
      setToast(msg);
      setStatusText(msg);
      return;
    }
    const account = nativeAddressInput.trim();
    if (!account) {
      const msg = trText(
        '当前地址未就绪，无法打开区块浏览器',
        'Current address is not ready; cannot open explorer'
      );
      setToast(msg);
      setStatusText(msg);
      return;
    }

    const tokenAddress = detailAsset.kind === 'token' ? detailAsset.tokenAddress : '';
    const url = buildExplorerUrlFromConfig(configuredExplorer, account, tokenAddress);
    if (!url) {
      const msg = trText(
        `当前网络 config 未配置区块浏览器链接: ${selectedNetwork}`,
        `Explorer URL is not configured for network: ${selectedNetwork}`
      );
      setToast(msg);
      setStatusText(msg);
      return;
    }

    window.open(url, '_blank', 'noopener,noreferrer');
    setStatusText(
      trText(
        `已打开区块浏览器：${selectedConfig.title} ${detailAsset.symbol || 'Asset'}`,
        `Opened explorer: ${selectedConfig.title} ${detailAsset.symbol || 'Asset'}`
      )
    );
  }

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

      <TopBar
        backendCanisterId={backendCanisterId}
        trText={trText}
        lang={lang}
        setLang={setLang}
        selectedNetwork={selectedNetwork}
        onSelectedNetworkChange={setSelectedNetwork}
        networkOptions={networkOptions}
        networkDisplayNames={networkDisplayNames}
        onLoginClick={handleLoginClick}
      />

      <AssetPanel
        selectedConfig={selectedConfig}
        trText={trText}
        nativeAddressInput={nativeAddressInput}
        nativeBalanceValue={nativeBalanceValue}
        nativeBalanceMeta={nativeBalanceMeta}
        tokenListCount={tokenListCount}
        tokenStartIndex={tokenStartIndex}
        selectedAssetRowKey={selectedAssetRowKey}
        visibleAssetItems={visibleAssetItems}
        tokenRowBalances={tokenRowBalances}
        onTokenListScroll={setTokenListScrollTop}
        openTokenDetail={openTokenDetail}
      />

      {detailAsset && (
        <AssetDetailModal
          detailAsset={detailAsset}
          selectedConfig={selectedConfig}
          trText={trText}
          closeTokenDetail={closeTokenDetail}
          nativeAddressInput={nativeAddressInput}
          detailBalanceValue={detailBalanceValue}
          detailBalanceMeta={detailBalanceMeta}
          handleOpenExplorerClick={handleOpenExplorerClick}
          tokenTransferTo={tokenTransferTo}
          setTokenTransferTo={setTokenTransferTo}
          tokenTransferAmount={tokenTransferAmount}
          setTokenTransferAmount={setTokenTransferAmount}
          isTokenSending={isTokenSending}
          handleTokenSendClick={handleTokenSendClick}
        />
      )}
    </div>
  );
}
