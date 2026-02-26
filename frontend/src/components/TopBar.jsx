import { NETWORK_CONFIG, fallbackNetworkConfig } from '../config/networks';

export default function TopBar({
  backendCanisterId,
  trText,
  lang,
  setLang,
  selectedNetwork,
  onSelectedNetworkChange,
  networkOptions,
  networkDisplayNames,
  onLoginClick
}) {
  return (
    <header className="topbar">
      <div className="brand">
        <div className="brand__eyebrow">AGENT WALLET CONTROL PLANE</div>
        <div className="brand__title">rustwalletforagent</div>
        <div className="brand__meta" title={backendCanisterId || ''}>
          <span className="brand__meta-label">{trText('后端 Canister ID', 'Backend Canister ID')}</span>
          <code className="brand__meta-value">{backendCanisterId || trText('未读取', 'Not loaded')}</code>
        </div>
      </div>

      <div className="topbar__actions">
        <div className="lang-toggle" role="group" aria-label={trText('语言切换', 'Language switch')}>
          <button
            type="button"
            className={`lang-toggle__btn${lang === 'zh' ? ' is-active' : ''}`}
            onClick={() => setLang('zh')}
            aria-pressed={lang === 'zh'}
          >
            中文
          </button>
          <button
            type="button"
            className={`lang-toggle__btn${lang === 'en' ? ' is-active' : ''}`}
            onClick={() => setLang('en')}
            aria-pressed={lang === 'en'}
          >
            EN
          </button>
        </div>

        <label className="network-picker">
          <span className="network-picker__label">{trText('网络', 'NETWORK')}</span>
          <select
            className="network-picker__select"
            value={selectedNetwork}
            onChange={(event) => onSelectedNetworkChange(event.target.value)}
            aria-label={trText('选择网络', 'Select network')}
          >
            {networkOptions.map((networkId) => {
              const cfg = NETWORK_CONFIG[networkId] || fallbackNetworkConfig(networkId);
              const displayName = networkDisplayNames[networkId] || cfg.title;
              return (
                <option key={networkId} value={networkId}>
                  {displayName}
                </option>
              );
            })}
          </select>
        </label>

        <button type="button" className="button button--ghost" onClick={onLoginClick}>
          {trText('登录', 'Login')}
        </button>
      </div>
    </header>
  );
}
