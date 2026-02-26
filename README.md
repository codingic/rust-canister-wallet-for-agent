# canister-wallet-for-agent

一个用 Rust 编写的 ICP canister 钱包后端，面向 AI Agent 提供多链资产地址申请、余额查询与转账接口。后续支持DEX

AI Agent 通过调用 后端canister与区块链交互。 AI Agent 不持有私钥，私钥在ICP 的子网，非常安全。

后端canister 有鉴权机制，只能由 Agent 控制后端canister的调用。

后端属于 Agent 的私有canister。其他任何人无权调用，后端canister会直接拒绝。

前端为控制台页面。前端供人类使用, 展示为多链钱包。 也可不部署前端。只部署后端给 Agent 使用。


icp 发送已测
icrc1 token 发送已测

evm 发送已测
erc20 发送已测

sol 发送已测
sol-token 发送未测

btc 未测
ton 未测
apt 未测
sui 未测
trx 未测
near 未测

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
- `internet_computer`
- `bitcoin`
- `solana`
- `solana_testnet`
- `tron`
- `ton_mainnet`
- `near_mainnet`
- `aptos_mainnet`
- `sui_mainnet`

说明：链相关接口命名统一按“网络名字（下划线化）”风格，例如 `ethereum_transfer_eth`、`solana_request_address`。

## 地址申请（真实实现）

已实现真实地址申请（management canister 公钥，接口均为无参）：

- EVM 系（同地址组，均复用 `eth::request_address()`）：
  - `ethereum_request_address`
  - `sepolia_request_address`
  - `base_request_address`
  - `bsc_request_address`
  - `arbitrum_request_address`
  - `optimism_request_address`
  - `avalanche_request_address`
  - `okx_request_address`
  - `polygon_request_address`
- 其他链：
  - `bitcoin_request_address`（Taproot / `bc1...`）
  - `internet_computer_request_address`（返回后端 canister principal，作为 ICP/ICRC 默认托管地址）
  - `solana_request_address`
  - `solana_testnet_request_address`
  - `tron_request_address`
  - `ton_mainnet_request_address`
  - `near_mainnet_request_address`
  - `aptos_mainnet_request_address`
  - `sui_mainnet_request_address`
- `internet_computer`：也支持统一地址接口 `internet_computer_request_address()`，便于 agent 走统一流程

地址派生规则（当前）：

- 使用 management canister 公钥
- `canister_id = null`（当前后端 canister）
- `derivation_path = []`（不带任何派生参数）


## 接口命名规则（显式）

- 地址申请（后端 canister）：`<network_prefix>_request_address`
- 转账（后端 canister）：`<network_prefix>_transfer_<asset_kind>`
- 余额（`allchain-api-jssdk`）：`<network_prefix>_get_balance_<asset_kind>`

这样可以避免一个接口同时承担原生币与 token 资产的歧义。
说明：后端对外 `*_get_balance_*` 接口已移除，Agent 应通过 `allchain-api-jssdk` 查询余额。


## JS Agent 技能（Skill）与 SDK

仓库内提供给其他 JS/TS Agent 使用本工程的技能文档与独立 SDK：

- `skills/canister-wallet-js-usage-for-agent.md`
- `deploy-canister-js-sdk-for-agent/`（部署与后端 canister 交互 SDK）
- `allchain-api-jssdk/`（链侧余额查询 SDK）

内容包括：

- 使用 `deploy-canister-js-sdk-for-agent`（封装 Actor 创建、Result 解包、动态方法名构建）
- `deploy-canister-js-sdk-for-agent/examples/*` 示例脚本（快速连通 / 发送 ETH / 添加 Token）
- 使用 `allchain-api-jssdk` 查询各链余额（方法名统一：`<network_prefix>_get_balance_<asset_kind>`）
- 必要时使用 `src/declarations/backend` 创建 `@dfinity/agent` Actor（raw fallback）
- `Result`（`Ok/Err`）返回值解析模式
- 基于 `wallet_networks()` 的 `shared_address_group` 做地址共享判断（例如 EVM 同地址）
- 按网络名动态拼接地址/余额/转账方法名
- 使用运行时配置接口 `configured_rpcs / set_configured_rpc / remove_configured_rpc`
- 使用 TokenList 动态接口 `configured_tokens / add_configured_token / remove_configured_token`

### OpenClaw 无人参与自举部署流程（canister-factory）

仓库已提供一套 JS 脚本骨架，支持 OpenClaw 自己生成身份、请求 `canister-factory` canister 创建 backend canister、安装 `backend.wasm`、并在生产模式首次自举 `owner`：

- `deploy-canister-js-sdk-for-agent/examples/bootstrap-openclaw-backend.js`（一体化流程）
- `deploy-canister-js-sdk-for-agent/examples/deploy-backend-canister.js`（仅安装/升级已有 canister）
- `deploy-canister-js-sdk-for-agent/src/identity-file.js`（加密 identity 文件，文本文件不包含明文私钥）
- `deploy-canister-js-sdk-for-agent/src/canister-factory.js`（调用 `canister-factory` canister 创建 canister）
- `deploy-canister-js-sdk-for-agent/src/deployer.js`（管理 canister install/upgrade/update_settings 封装）

流程（最简单可跑版本）：

1. OpenClaw 生成一个加密文本文件（例如 `key.pem`），文件中不包含明文私钥；人输入密码用于加密/解密
2. OpenClaw 从该加密文件恢复 `Ed25519` identity（ICP 调用身份）
3. OpenClaw 调用生产环境 `canister_factory` canister 请求为 `caller()` 创建一个 backend canister（controller = OpenClaw identity）
4. OpenClaw 使用该 identity 部署 `backend.wasm`
5. 部署完成后（生产模式首次）调用 `rotate_owner(agent_principal)` 自举 `owner`  成为唯一控制钱包的identity

预先准备：

- `backend.wasm`
- 一个有 cycles 的 `canister_factory` canister（由你部署，供用户免费/配额使用）

`canister_factory` 推荐接口（当前 JS 脚本按这个接口约定）：

- `create_canister_for_caller : (record { cycles : opt nat64 }) -> (variant { Ok : principal; Err : ... })`

要求：

- `canister_factory` 必须用 `caller()` 作为新 canister 的 controller（不要信客户端传 controller）
- `canister_factory` 需要有 cycles，并且应实现限额/速率限制，避免被刷爆

示例运行（首次创建 + 部署 + owner 自举）：

```bash
CANISTER_FACTORY_CANISTER_ID=<canister_factory_canister_id> \
IC_HOST=https://icp-api.io \
WASM_PATH=./target/wasm32-unknown-unknown/release/backend.wasm \
node deploy-canister-js-sdk-for-agent/examples/bootstrap-openclaw-backend.js
```

脚本会在本地保存：

- 加密 identity 文件（默认：`./.openclaw/openclaw-agent.key.pem`）
- 部署状态文件（默认：`./.openclaw/backend-deploy-state.json`，保存 `backend_canister_id` 等）

### canister-factory（Factory 后端）

新增 `/canister-factory` 后端 canister，用于为调用者创建独立 backend canister（controller 自动设置为 `caller()`），供 OpenClaw 等 agent 无人参与自举部署使用。

主要接口：

- `create_canister_for_caller({ cycles? })`：按 `caller()` 创建 canister，附加可选 cycles
- `rotate_owner(principal)`：factory 自身 owner 自举/轮换（用于管理工厂参数）
- `set_paused(bool)`：暂停/恢复创建
- `set_public_create_enabled(bool)`：开启/关闭公共创建
- `set_default_extra_cycles(nat64)`：设置默认附加 cycles
- `set_max_extra_cycles_per_create(nat64)`：设置单次创建最大 cycles
- `set_max_canisters_per_caller(nat32)`：设置每个 caller 的创建配额
- `reset_caller_quota(principal)`：重置指定 caller 已创建计数
- `service_info()` / `my_created_count()`：查询工厂状态和调用者配额

特点：

- 使用 management canister `create_canister_with_extra_cycles`
- `controllers = [caller()]`（不信任客户端传 controller）
- 内置基础配额、开关、统计，并支持 stable upgrade 持久化
- 与 `deploy-canister-js-sdk-for-agent/examples/bootstrap-openclaw-backend.js` 直接对接

## allchain-api-jssdk（余额查询 SDK）

新增 `allchain-api-jssdk/`，专门给 JS Agent 查询各链余额使用。

特点：

- 按功能模块拆分：`config / core / chains / utils`
- 方法名统一：`<network_prefix>_get_balance_<asset_kind>`
- 覆盖 EVM、Bitcoin、ICP/ICRC、Solana/SPL、TRON/TRC20、TON/Jetton、NEAR/NEP-141、Aptos、Sui
- 返回字段尽量对齐原后端 `BalanceResponse` 结构（`network/account/token/amount/decimals/message`）

示例：

```js
import { createAllChainApiClient } from './allchain-api-jssdk/src/index.js';

const chainApi = createAllChainApiClient();
const eth = await chainApi.ethereum_get_balance_eth({
  account: '0x0000000000000000000000000000000000000000'
});
```

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

- 多链地址申请、转账、配置接口统一在 `backend/api.rs`
- 多链余额查询逻辑实现保留在各链模块（后端对外 `*_get_balance_*` API 已移除）
- Agent 余额查询应使用 `allchain-api-jssdk`
- 外部 HTTP RPC 统一从 `backend/outcall.rs` 走，便于后续加重试/transform/审计
- 网络名统一使用 `types::networks::*` 常量
- EVM 系网络共享地址组信息通过 `wallet_networks()` 对外暴露（供 agent 直接消费）
- 前端已按模块拆分（`frontend/src/config`、`frontend/src/api`、`frontend/src/components`）
