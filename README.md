# canisterwalletforagent

一个用 Rust 编写的 ICP canister 钱包后端，面向 AI Agent 提供多链资产地址申请、余额查询与转账接口；前端为 React 控制台页面。

## 项目定位

- 本工程不负责 `controller` / `agent` 私钥生成与托管
- 本工程负责 canister 接口（Candid + Rust 实现），供 agent 按 `caller()` 调用
- 当前优先级是先把多链功能打通，鉴权策略后续再收紧（owner/agent/policy）

## 功能概览（当前）

### 后端（Rust canister）

- `backend/api.rs`：所有 canister 对外接口集中定义
- `backend/{...}.rs`：按链拆分模块（EVM / BTC / ICP / Solana / TRON / TON / NEAR / Aptos / Sui 等）
- `backend/config/*`：RPC、Token 列表、区块浏览器配置（前端通过接口读取）
- `pre_upgrade` / `post_upgrade`：状态持久化
- `backend/backend.did`：Candid 接口导出

### 前端（React）

- 顶部显示 `Backend Canister ID`
- 网络选择（与后端 `supported_networks()` / `wallet_networks()` 对齐）
- 自动申请当前网络地址并自动查询主币余额（支持的链）
- Token 列表从后端 `configured_tokens(network)` 读取
- Token 虚拟列表（主币在第一个位置）
- 币种详情全屏界面（资产信息 + 发送表单）
- 区块浏览器按钮（浏览器链接从后端 `configured_explorer(network)` 读取）

## 网络命名（统一）

对外统一使用“网络名字”，不是主币符号，例如：

- `ethereum`
- `sepolia`
- `base`
- `bsc`
- `arbitrum`
- `optimism`
- `avalanche`
- `okx`
- `polygon`
- `internet-computer`
- `bitcoin`
- `solana`
- `solana-testnet`
- `tron`
- `ton-mainnet`
- `near-mainnet`
- `aptos-mainnet`
- `sui-mainnet`

说明：接口名仍保留历史前缀（例如 `eth_*`、`sol_*`），但请求/响应里的 `network` 与前端网络选择统一为上述网络名字。

## 地址申请（真实实现）

已实现真实地址申请（management canister 公钥）：

- `eth_request_address`（EVM 地址，`secp256k1`）
- `sepolia_request_address`（EVM 地址，`secp256k1`）
- `btc_request_address`（Taproot 地址，`bip340secp256k1`）
- `sol_request_address`（Solana 地址，`ed25519`）
- `solana_testnet_request_address`（Solana Testnet 地址，`ed25519`）

地址派生规则（当前）：

- 使用 management canister 公钥
- `canister_id = null`（当前后端 canister）
- `derivation_path = []`（不带任何派生参数）

## 接口命名规则（显式）

- 余额：`<network_prefix>_get_balance_<asset_kind>`
- 转账：`<network_prefix>_transfer_<asset_kind>`

这样可以避免一个接口同时承担原生币与 token 资产的歧义。

## 已实现的“真实”链上功能

### EVM 系（真实余额 + 真实发送）

已实现原生币余额、ERC20 余额、原生币发送、ERC20 发送（RPC + canister 签名 + 广播）：

- `ethereum`
- `sepolia`
- `base`
- `bsc`
- `arbitrum`
- `optimism`
- `avalanche`
- `okx`
- `polygon`

实现方式（后端）：

- 余额：RPC `eth_getBalance` / `eth_call(balanceOf)`
- 发送：构造交易 -> `sign_with_ecdsa` -> `eth_sendRawTransaction`

### Solana / Solana Testnet（当前已实现）

- `SOL` 主币余额（真实 RPC `getBalance`）
- `SOL` 主币发送（真实交易构造 + `sign_with_schnorr(ed25519)` + `sendTransaction`）
- `SPL` Token 发送（`TransferChecked`，真实广播）

已支持网络：

- `solana`
- `solana-testnet`

注意（SPL 发送）：

- `To 地址` 按“钱包地址”处理（不是 token account）
- 后端会用 `getTokenAccountsByOwner` 查目标地址对应 mint 的 token account
- 如果对方还没有该 mint 的 token account，发送会失败（当前不自动创建 ATA）

## 仍为骨架/占位的部分（当前）

以下链的大部分余额/转账逻辑仍是 scaffold（已完成接口形状与前后端路由）：

- `bitcoin`
- `internet-computer`
- `tron`
- `ton-mainnet`
- `near-mainnet`
- `aptos-mainnet`
- `sui-mainnet`

以及：

- `solana` / `solana-testnet` 的 `SPL` 余额查询仍未实现（发送已实现）

## 重要后端配置接口（前端使用）

- `wallet_networks()`：网络列表与基础能力信息（主币符号、是否支持发送/余额、默认 RPC）
- `configured_tokens(network)`：当前网络 token 列表
- `configured_explorer(network)`：区块浏览器 URL 模板（地址页 / token 页）

## 构建与部署

### 后端

```bash
cargo check -p backend
cargo build --target wasm32-unknown-unknown --release -p backend
```

更新 Candid：

```bash
candid-extractor target/wasm32-unknown-unknown/release/backend.wasm > backend/backend.did
```

### 前端

```bash
npm run build --prefix frontend
```

前端构建脚本会执行：

- `dfx generate backend`
- `vite build`

### 本地部署（示例）

```bash
dfx start --background --clean
dfx deploy backend
npm run build --prefix frontend
dfx deploy frontend
```

## 当前状态说明

- 鉴权逻辑当前仍是 placeholder（后续会收紧到 owner/agent/policy）
- EVM 系核心能力已打通（余额/发送）
- Solana / Solana Testnet 已打通地址、SOL 余额、SOL 发送、SPL 发送
- 其余链逐步从 scaffold 迁移到真实链 RPC / ledger 调用

