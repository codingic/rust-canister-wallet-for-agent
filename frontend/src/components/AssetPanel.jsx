import { TOKEN_VLIST_ROW_HEIGHT } from '../config/networks';

export default function AssetPanel({
  selectedConfig,
  trText,
  nativeAddressInput,
  nativeBalanceValue,
  nativeBalanceMeta,
  tokenListCount,
  tokenStartIndex,
  selectedAssetRowKey,
  visibleAssetItems,
  tokenRowBalances,
  onTokenListScroll,
  openTokenDetail
}) {
  return (
    <main className="layout layout--single">
      <section className="layout__main">
        <section className="asset-grid" aria-label={trText('资产卡片', 'Asset cards')}>
          <article className="panel asset-card asset-card--native">
            <header className="asset-card__head">
              <div>
                <p className="asset-card__eyebrow">NATIVE ASSET</p>
                <h2>{selectedConfig.nativeSymbol}</h2>
              </div>
              <span className="pill pill--glow">{trText('主资产', 'Primary')}</span>
            </header>

            <div className="asset-card__row">
              <div className="asset-card__label">{trText('地址', 'Address')}</div>
              <div className="mono-block">{nativeAddressInput.trim() || '--'}</div>
            </div>

            <div className="asset-card__row">
              <div className="asset-card__label">{trText('余额', 'Balance')}</div>
              <div className="asset-card__balance">{nativeBalanceValue}</div>
              <div className="asset-card__sub">{nativeBalanceMeta}</div>
            </div>

            {selectedConfig.showToken && (
              <div className="asset-card__row token-vlist">
                <div className="token-vlist__header">
                  <div className="asset-card__label token-vlist__title">{trText('Token 列表', 'Token List')}</div>
                  <span className="pill">
                    {tokenListCount
                      ? trText(`${tokenListCount} 项`, `${tokenListCount} items`)
                      : trText('无资产', 'No assets')}
                  </span>
                </div>

                {tokenListCount ? (
                  <div
                    className="token-vlist__viewport"
                    onScroll={(event) => onTokenListScroll(event.currentTarget.scrollTop)}
                    role="list"
                    aria-label={trText(`${selectedConfig.title} Token 列表`, `${selectedConfig.title} token list`)}
                  >
                    <div className="token-vlist__spacer" style={{ height: `${tokenListCount * TOKEN_VLIST_ROW_HEIGHT}px` }}>
                      {visibleAssetItems.map((asset, offset) => {
                        const index = tokenStartIndex + offset;
                        const isActive = asset.rowKey === selectedAssetRowKey;
                        const rowBalance = asset.kind === 'native' ? null : tokenRowBalances[asset.tokenAddress];
                        const rowBalanceText =
                          asset.kind === 'native'
                            ? nativeBalanceValue
                            : rowBalance?.phase === 'loading'
                              ? trText('查询中...', 'Loading...')
                              : rowBalance?.phase === 'error'
                                ? trText('查询失败', 'Query failed')
                                : rowBalance?.amount || trText('未查询', 'Not queried');
                        const rowBalanceMeta =
                          asset.kind === 'native'
                            ? nativeBalanceMeta
                            : rowBalance?.phase === 'error'
                              ? rowBalance.error
                              : rowBalance?.message ||
                                (rowBalance?.pending
                                  ? 'pending=true'
                                  : `${trText('精度', 'decimals')}: ${String(asset.decimals ?? '--')}`);
                        return (
                          <button
                            key={asset.rowKey}
                            type="button"
                            className={`token-vlist__item${isActive ? ' token-vlist__item--active' : ''}`}
                            style={{ transform: `translateY(${index * TOKEN_VLIST_ROW_HEIGHT}px)` }}
                            onClick={() => openTokenDetail(asset)}
                            role="listitem"
                            aria-pressed={isActive}
                          >
                            <div className="token-vlist__item-main">
                              <div className="token-vlist__symbol">{asset.symbol || 'TOKEN'}</div>
                              <div className="token-vlist__name">
                                {asset.kind === 'native'
                                  ? trText('原生资产', 'Native Asset')
                                  : asset.name || trText('未命名 Token', 'Unnamed Token')}
                              </div>
                            </div>
                            <div className="token-vlist__item-meta">
                              <div className="token-vlist__addr">
                                {asset.kind === 'native'
                                  ? `${trText('地址', 'Address')}: ${nativeAddressInput.trim() || '--'}`
                                  : `${trText('合约', 'Contract')}: ${asset.tokenAddress}`}
                              </div>
                              <div className="token-vlist__decimals">
                                {trText('精度', 'Decimals')}: {String(asset.decimals ?? '--')}
                              </div>
                              <div className="token-vlist__balance">
                                {trText('余额', 'Balance')}: {rowBalanceText}
                              </div>
                              <div className="token-vlist__balance-meta">{rowBalanceMeta}</div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ) : (
                  <div className="mono-block">
                    {trText('当前网络 config 未配置 Token', 'No tokens configured for this network')}
                  </div>
                )}
              </div>
            )}
          </article>
        </section>
      </section>
    </main>
  );
}
