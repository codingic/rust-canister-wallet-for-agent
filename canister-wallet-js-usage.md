---
name: icp-rust-agent-wallet-js-usage
description: 用 JavaScript 调用已部署的 rust-canister-wallet-for-agent canister（ICP 上多链 Agent 钱包）。包括完整 JS 示例代码、接口说明、查询余额、转账 ICP/EVM/Solana 等操作。适用于 OpenClaw 或任何 Node.js/浏览器环境。
parameters:
  type: object
  properties:
    topic:
      type: string
      enum: [overview, install-deps, get-canister-id, query-balance, transfer-icp, transfer-evm, full-example, troubleshoot]
      description: 指定想看的具体部分，或 overview 获取整体说明
  required: [topic]
---

## 使用说明
当用户问“怎么用 JS 调用 rust-canister-wallet-for-agent？”“OpenClaw 怎么调用 ICP 上的 Agent wallet？”时，调用此 skill。

核心信息：
- 这个 canister 是一个 Rust 写的 ICP 后端钱包，支持多链（ICP、Bitcoin、EVM、Solana 等）
- 前端是 React 控制台（可选）
- JS 调用主要靠 @dfinity/agent + candid 接口
- 需要 canister ID（部署后获得）

步骤与代码示例：

1. overview
   - 支持的主要方法（Candid 接口）：
     - wallet_networks() → 返回支持的链列表 (vec text)
     - wallet_balance(network: text) → 查询余额 (nat)
     - wallet_transfer(network: text, to: text, amount: nat) → 转账
     - sign_message(message: blob) → 签名
   - 单位：ICP 用 e8s (1 ICP = 10^8 e8s)，EVM 用 wei

2. install-deps
   - 在你的 Node.js 项目或 OpenClaw 扩展中安装依赖：
     ```bash
     npm install @dfinity/agent @dfinity/principal candid
     ```
   - 如果是浏览器环境，还需：
     ```bash
     npm install @dfinity/identity
     ```

3. get-canister-id
   - 部署后获取 canister ID：
     ```bash
     dfx canister id backend --network ic
     ```
   - 把 ID 保存到环境变量或配置文件中

4. query-balance（查询余额示例）
   ```javascript
   import { Actor, HttpAgent } from '@dfinity/agent';
   import { Principal } from '@dfinity/principal';
   import { idlFactory } from './declarations/backend'; // 从 dfx 生成的 candid 文件
   import { _SERVICE } from './declarations/backend/backend.did'; // 类型定义

   async function getBalance(network = 'icp') {
     const agent = new HttpAgent({ host: 'https://ic0.app' });
     // 如果是本地测试：agent.fetchRootKey();

     const canisterId = Principal.fromText('你的 canister ID 这里填');
     const wallet = Actor.createActor<_SERVICE>(idlFactory, {
       agent,
       canisterId,
     });

     const balance = await wallet.wallet_balance(network);
     console.log(`${network} 余额: ${Number(balance) / 1e8} ICP`); // e8s 转 ICP
     return balance;
   }

   getBalance('icp').catch(console.error);