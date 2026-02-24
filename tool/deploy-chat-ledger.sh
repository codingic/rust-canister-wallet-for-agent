#!/usr/bin/env bash
set -euo pipefail

# 单独部署 CHAT token ledger（复用本地 ledger canister wasm）
# 默认把初始余额发到 backend canister 对应账户，前端资产页会直接显示该余额。

LEDGER_CANISTER="chat_ledger"
DEPLOY_MODE="reinstall"   # 可选: install | reinstall | upgrade

MINTING_ACCOUNT="949f7b6fb6b27944bd230ce528303fb7d72e333a9a8f414be6dc41cd93f177b7"
TOKEN_SYMBOL="CHAT"
TOKEN_NAME="Chat"
TRANSFER_FEE_E8S="10000"
INITIAL_BALANCE_E8S="100000000000"
INITIAL_HOLDER_CANISTER_ALIAS="${INITIAL_HOLDER_CANISTER_ALIAS:-backend}"
INITIAL_HOLDER_ACCOUNT="${INITIAL_HOLDER_ACCOUNT:-}"

run_dfx() {
  TERM=xterm-256color NO_COLOR=1 CLICOLOR=0 DFX_DISABLE_COLOR=1 dfx "$@"
}

if [[ -z "${INITIAL_HOLDER_ACCOUNT}" ]]; then
  if ! INITIAL_HOLDER_ACCOUNT="$(run_dfx ledger account-id --of-canister "${INITIAL_HOLDER_CANISTER_ALIAS}" -q 2>/dev/null | tr -d '[:space:]')"; then
    echo "Warning: failed to resolve account-id for canister '${INITIAL_HOLDER_CANISTER_ALIAS}', fallback to minting account." >&2
    INITIAL_HOLDER_ACCOUNT="${MINTING_ACCOUNT}"
  fi
fi

LEDGER_ARG="(variant { Init = record {
  send_whitelist = vec {};
  token_symbol = opt \"${TOKEN_SYMBOL}\";
  transfer_fee = opt record { e8s = ${TRANSFER_FEE_E8S} : nat64 };
  minting_account = \"${MINTING_ACCOUNT}\";
  transaction_window = null;
  max_message_size_bytes = null;
  icrc1_minting_account = null;
  archive_options = null;
  initial_values = vec {
    record {
      \"${INITIAL_HOLDER_ACCOUNT}\";
      record { e8s = ${INITIAL_BALANCE_E8S} : nat64 }
    }
  };
  token_name = opt \"${TOKEN_NAME}\";
  feature_flags = opt record { icrc2 = true }
} })"

echo "Deploying ${LEDGER_CANISTER} with mode=${DEPLOY_MODE} ..."
run_dfx \
  --log file \
  --logfile /tmp/dfx-deploy-chat-ledger.log \
  deploy "${LEDGER_CANISTER}" --mode "${DEPLOY_MODE}" --argument "${LEDGER_ARG}"

echo "CHAT ledger deploy finished."
echo "Initial CHAT holder account-id: ${INITIAL_HOLDER_ACCOUNT}"
run_dfx canister id "${LEDGER_CANISTER}"
