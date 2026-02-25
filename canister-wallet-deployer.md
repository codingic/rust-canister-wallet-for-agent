---
name: self-icp-wallet-deployer
description: 指导并半自动化部署 Rust canister wallet 到 ICP，包括先创建 Internet Identity，然后克隆 https://github.com/beyondapplei/rust-canister-wallet-for-agent 并部署。适合有本地 dfx 和 Rust 环境的开发者。
parameters:
  type: object
  properties:
    step:
      type: string
      enum: [create-identity, setup-env, clone-repo, build-deploy, full-guide]
      description: 指定执行哪个步骤，或 full-guide 走全流程
    principal_id:
      type: string
      description: 如果已有 ICP Principal，可手动传入
    cycles_amount:
      type: number
      description: 部署需要的 cycles 数量（默认建议 10T+）
  required: [step]
---

## 使用说明
当用户说“部署那个 rust-canister-wallet-for-agent 到 ICP”或“用 ICP identity 部署 Agent wallet”时，调用此 skill。

执行逻辑（Reason-Act-Observe 循环）：

1. **检查前提**：
   - 本地是否安装 dfx（运行 dfx --version）
   - 是否安装 Rust + wasm32-unknown-unknown target（rustup target add wasm32-unknown-unknown）
   - 是否有 ICP cycles（主网部署需要充值）
   - 是否有 Internet Identity（如果没有，先引导创建）

2. **步骤分解**（按 step 参数执行）：

   - create-identity：
     - 打开浏览器到 https://identity.ic0.app/
     - 指导用户手动创建 passkey-based Internet Identity（Agent 无法自动生成硬件 passkey，但可描述步骤）
     - 或者用 dfx identity new my-agent-id --storage-mode plaintext 生成软件身份（不推荐主网）
     - 保存 Principal 到文件 ~/.openclaw/workspace/icp_principal.txt
     - 更新 IDENTITY.md：添加“我现在有 ICP Principal: [你的Principal]”

   - setup-env：
     - 运行命令：
       dfx --version
       rustc --version
       cargo --version
       rustup target list | grep wasm
     - 如果缺任何一项，输出安装命令并暂停

   - clone-repo：
     - git clone https://github.com/beyondapplei/rust-canister-wallet-for-agent.git
     - cd rust-canister-wallet-for-agent
     - 建议阅读 README.md（用 browser skill 打开 https://github.com/beyondapplei/rust-canister-wallet-for-agent/blob/main/README.md）

   - build-deploy：
     - dfx canister create --all（或指定 canister 名）
     - dfx build
     - dfx deploy --network ic（主网）或 --network local（测试）
     - 如果需要 cycles：dfx wallet --network ic receive <你的cycles来源>
     - 记录部署后的 canister ID

   - full-guide：
     - 按顺序执行以上所有步骤
     - 每步输出详细命令 + 预期输出
     - 最后回复：部署成功后的 canister ID、如何调用、如何用这个 wallet 让 Agent 持有 ICP 资产

3. **安全提醒**：
   - 主网部署消耗 cycles（不可逆），先用本地 replica 测试（dfx start --background）
   - 不要把私钥/种子传给我或任何外部
   - 部署后更新 SOUL.md / MEMORY.md 记录这个 wallet canister ID

4. **输出格式**：
   - 每步用编号列表
   - 命令用 ```bash 代码块
   - 完成后问：需要我帮你执行下一步吗？或修改哪个部分？





