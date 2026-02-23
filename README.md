# canisterwalletforagent

一个用 Rust 编写的 ICP canister 钱包后端，面向 AI Agent 提供多链资产查询与转账接口（当前大部分链为功能骨架），前端为 React 控制台页面。

## 项目定位

- 本工程不负责 `controller` / `agent` 私钥生成与托管
- 本工程负责 canister 接口（Candid + Rust 实现），供 agent 按 `caller()` 调用
- 当前优先级是先打通接口形状与链路，鉴权策略后续收紧

## 当前已实现内容

### 1. 后端（Rust canister）模块化结构

- `backend/api.rs`：统一对外接口入口（所有 canister 方法集中）
- `backend/{btc,eth,base,bsc,arb,op,avax,okb,polygon,icp,sol,trx,ton,near,aptos,sui}.rs`：按链拆分
- `backend/types.rs` / `backend/error.rs` / `backend/state.rs`
- `pre_upgrade` / `post_upgrade` 已接入
- `backend/backend.did` 由 `candid-extractor` 生成

### 2. 地址申请（真实后端逻辑）

已支持以下地址申请（非模拟）：

- `eth_request_address`
- `btc_request_address`
- `sol_request_address`

说明：

- `ETH` 使用 management canister `ecdsa_public_key` 推导地址
- `BTC` 使用 `schnorr_public_key(bip340secp256k1)` 推导 Taproot 地址
- `SOL` 使用 `schnorr_public_key(ed25519)` 推导地址

### 3. 多链余额/转账接口（显式命名）

接口命名规则统一为：

- 余额：`<network>_get_balance_<asset_kind>`
- 转账：`<network>_transfer_<asset_kind>`

这样可以避免一个接口同时承担原生币与 token 资产的歧义。

## 已接入网络与接口命名（当前）

### EVM 系

- `eth`
  - `eth_get_balance_eth`
  - `eth_get_balance_erc20`
  - `eth_transfer_eth`
  - `eth_transfer_erc20`
- `base`
  - `base_get_balance_eth`
  - `base_get_balance_erc20`
  - `base_transfer_eth`
  - `base_transfer_erc20`
- `bsc`
  - `bsc_get_balance_bnb`
  - `bsc_get_balance_bep20`
  - `bsc_transfer_bnb`
  - `bsc_transfer_bep20`
- `arb`
  - `arb_get_balance_eth`
  - `arb_get_balance_erc20`
  - `arb_transfer_eth`
  - `arb_transfer_erc20`
- `op`
  - `op_get_balance_eth`
  - `op_get_balance_erc20`
  - `op_transfer_eth`
  - `op_transfer_erc20`
- `avax`
  - `avax_get_balance_avax`
  - `avax_get_balance_erc20`
  - `avax_transfer_avax`
  - `avax_transfer_erc20`
- `okb`
  - `okb_get_balance_okb`
  - `okb_get_balance_erc20`
  - `okb_transfer_okb`
  - `okb_transfer_erc20`
- `polygon`
  - `polygon_get_balance_pol`
  - `polygon_get_balance_erc20`
  - `polygon_transfer_pol`
  - `polygon_transfer_erc20`

### 其他链

- `btc`
  - `btc_get_balance_btc`
  - `btc_transfer_btc`
- `icp`
  - `icp_get_balance_icp`
  - `icp_get_balance_icrc`
  - `icp_transfer_icp`
  - `icp_transfer_icrc`
- `sol`
  - `sol_get_balance_sol`
  - `sol_get_balance_spl`
  - `sol_transfer_sol`
  - `sol_transfer_spl`
- `trx`
  - `trx_get_balance_trx`
  - `trx_get_balance_trc20`
  - `trx_transfer_trx`
  - `trx_transfer_trc20`
- `ton`
  - `ton_get_balance_ton`
  - `ton_get_balance_jetton`
  - `ton_transfer_ton`
  - `ton_transfer_jetton`
- `near`
  - `near_get_balance_near`
  - `near_get_balance_nep141`
  - `near_transfer_near`
  - `near_transfer_nep141`
- `aptos`
  - `aptos_get_balance_apt`
  - `aptos_get_balance_token`
  - `aptos_transfer_apt`
  - `aptos_transfer_token`
- `sui`
  - `sui_get_balance_sui`
  - `sui_get_balance_token`
  - `sui_transfer_sui`
  - `sui_transfer_token`

## 前端（React 控制台）

- 顶部显示 `Backend Canister ID`
- 网络选择下拉框（与后端 `supported_networks()` 对齐）
- 按网络显示原生资产地址 / Token 地址输入
- 点击“刷新余额”按网络和 token 类型自动路由到对应后端接口
- 当前不使用模拟余额数据（展示后端真实返回）

## 运行与构建

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

## 当前状态说明（重要）

- 多链余额/转账接口大部分仍是骨架实现（返回 `pending` 或 scaffold 响应）
- 鉴权逻辑当前为 placeholder（后续会收紧到 owner/agent 模型）
- 已有接口形状、命名规范、前后端联动与 Candid 导出流程

## 下一步建议

- 为 EVM 系链抽统一实现（减少 `eth/base/bsc/arb/op/avax/okb/polygon` 重复）
- 为更多链补 `request_address`
- 收紧鉴权（owner / agent / policy）
- 逐步替换 scaffold 为真实链 RPC / ledger 调用
