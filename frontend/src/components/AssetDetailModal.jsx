export default function AssetDetailModal({
  detailAsset,
  selectedConfig,
  trText,
  closeTokenDetail,
  nativeAddressInput,
  detailBalanceValue,
  detailBalanceMeta,
  handleOpenExplorerClick,
  tokenTransferTo,
  setTokenTransferTo,
  tokenTransferAmount,
  setTokenTransferAmount,
  isTokenSending,
  handleTokenSendClick
}) {
  return (
    <div className="token-detail-modal" role="dialog" aria-modal="true" aria-label={trText('资产详情', 'Asset detail')}>
      <div className="token-detail-modal__backdrop" onClick={closeTokenDetail} aria-hidden="true" />
      <section className="panel token-detail-modal__panel">
        <div className="token-detail-modal__shell">
          <header className="token-detail-modal__head">
            <div>
              <p className="asset-card__eyebrow">ASSET DETAIL</p>
              <h2>
                {detailAsset.symbol || 'TOKEN'}{' '}
                <span>{detailAsset.kind === 'native' ? 'Native Asset' : detailAsset.name || ''}</span>
              </h2>
            </div>
            <div className="token-detail-modal__head-actions">
              <span className="pill">{selectedConfig.title}</span>
              <span className={`pill ${detailAsset.kind === 'native' ? 'pill--glow' : ''}`}>
                {detailAsset.kind === 'native' ? trText('原生', 'Native') : 'Token'}
              </span>
              <button type="button" className="button button--ghost" onClick={closeTokenDetail}>
                {trText('关闭', 'Close')}
              </button>
            </div>
          </header>

          <div className="token-detail-modal__body">
            <section className="token-detail-card">
              <div className="token-detail-card__title">{trText('资产信息', 'Asset Info')}</div>

              <div className="token-detail-kv">
                <div className="asset-card__label">{trText('接收地址', 'Receive Address')}</div>
                <div className="mono-block">
                  {nativeAddressInput.trim() || trText('未获取到当前钱包地址', 'Wallet address not ready')}
                </div>
              </div>

              <div className="token-detail-kv">
                <div className="asset-card__label">
                  {detailAsset.kind === 'native'
                    ? trText('资产类型', 'Asset Type')
                    : trText('Token 合约地址', 'Token Contract')}
                </div>
                <div className="mono-block">
                  {detailAsset.kind === 'native'
                    ? trText('原生币（无合约地址）', 'Native asset (no contract address)')
                    : detailAsset.tokenAddress}
                </div>
              </div>

              <div className="token-detail-stats">
                <div className="token-detail-stat">
                  <div className="asset-card__label">{trText('精度', 'Decimals')}</div>
                  <div className="mono-block">{String(detailAsset.decimals ?? '--')}</div>
                </div>
                <div className="token-detail-stat token-detail-stat--balance">
                  <div className="asset-card__label">{trText('余额', 'Balance')}</div>
                  <div className="mono-block token-detail-stat__balance">{detailBalanceValue}</div>
                  <div className="asset-card__sub">{detailBalanceMeta}</div>
                </div>
              </div>

              <div className="token-detail-card__hint">
                {trText(
                  '当前地址与币种信息来自后端 canister 接口与 config 配置。',
                  'Address and asset metadata come from backend canister APIs and config.'
                )}
              </div>

              <div className="token-detail-card__actions">
                <button type="button" className="button button--ghost" onClick={handleOpenExplorerClick}>
                  {trText('区块浏览器查看', 'Open Explorer')}
                </button>
              </div>
            </section>

            <section className="token-detail-card token-detail-card--send">
              <div className="token-detail-card__title">{trText('发送交易', 'Send Transaction')}</div>

              <label className="token-detail-modal__field">
                <span className="asset-card__label">{trText('To 地址', 'To Address')}</span>
                <input
                  value={tokenTransferTo}
                  onChange={(event) => setTokenTransferTo(event.target.value)}
                  placeholder={trText('请输入接收方地址', 'Enter recipient address')}
                />
              </label>

              <label className="token-detail-modal__field">
                <span className="asset-card__label">{trText('数量', 'Amount')}</span>
                <input
                  value={tokenTransferAmount}
                  onChange={(event) => setTokenTransferAmount(event.target.value)}
                  placeholder={trText(
                    `请输入 ${detailAsset.symbol || 'Asset'} 数量`,
                    `Enter ${detailAsset.symbol || 'Asset'} amount`
                  )}
                />
              </label>

              <div className="token-detail-send-preview">
                <div className="token-detail-send-preview__row">
                  <span>{trText('网络', 'Network')}</span>
                  <strong>{selectedConfig.title}</strong>
                </div>
                <div className="token-detail-send-preview__row">
                  <span>{trText('资产', 'Asset')}</span>
                  <strong>{detailAsset.symbol || 'Asset'}</strong>
                </div>
                <div className="token-detail-send-preview__row">
                  <span>From</span>
                  <code>{nativeAddressInput.trim() || '--'}</code>
                </div>
              </div>

              <div className="token-detail-modal__actions">
                <button
                  type="button"
                  className="button button--primary"
                  onClick={handleTokenSendClick}
                  disabled={isTokenSending}
                >
                  {isTokenSending ? trText('发送中...', 'Sending...') : trText('发送', 'Send')}
                </button>
              </div>
            </section>
          </div>
        </div>
      </section>
    </div>
  );
}
