export const idlFactory = ({ IDL }) => {
  const AddConfiguredTokenRequest = IDL.Record({
    'token_address' : IDL.Text,
    'network' : IDL.Text,
  });
  const ConfiguredTokenResponse = IDL.Record({
    'decimals' : IDL.Nat64,
    'name' : IDL.Text,
    'token_address' : IDL.Text,
    'network' : IDL.Text,
    'symbol' : IDL.Text,
  });
  const WalletError = IDL.Variant({
    'Internal' : IDL.Text,
    'Paused' : IDL.Null,
    'InvalidInput' : IDL.Text,
    'Forbidden' : IDL.Null,
    'Unimplemented' : IDL.Record({
      'network' : IDL.Text,
      'operation' : IDL.Text,
    }),
  });
  const Result = IDL.Variant({
    'Ok' : ConfiguredTokenResponse,
    'Err' : WalletError,
  });
  const BalanceRequest = IDL.Record({
    'token' : IDL.Opt(IDL.Text),
    'account' : IDL.Text,
  });
  const BalanceResponse = IDL.Record({
    'decimals' : IDL.Opt(IDL.Nat8),
    'token' : IDL.Opt(IDL.Text),
    'pending' : IDL.Bool,
    'network' : IDL.Text,
    'block_ref' : IDL.Opt(IDL.Text),
    'message' : IDL.Opt(IDL.Text),
    'account' : IDL.Text,
    'amount' : IDL.Opt(IDL.Text),
  });
  const Result_1 = IDL.Variant({ 'Ok' : BalanceResponse, 'Err' : WalletError });
  const AddressResponse = IDL.Record({
    'network' : IDL.Text,
    'message' : IDL.Opt(IDL.Text),
    'address' : IDL.Text,
    'key_name' : IDL.Text,
    'public_key_hex' : IDL.Text,
  });
  const Result_2 = IDL.Variant({ 'Ok' : AddressResponse, 'Err' : WalletError });
  const TransferRequest = IDL.Record({
    'to' : IDL.Text,
    'token' : IDL.Opt(IDL.Text),
    'metadata' : IDL.Vec(IDL.Tuple(IDL.Text, IDL.Text)),
    'from' : IDL.Opt(IDL.Text),
    'memo' : IDL.Opt(IDL.Text),
    'nonce' : IDL.Opt(IDL.Text),
    'amount' : IDL.Text,
  });
  const TransferResponse = IDL.Record({
    'tx_id' : IDL.Opt(IDL.Text),
    'network' : IDL.Text,
    'message' : IDL.Text,
    'accepted' : IDL.Bool,
  });
  const Result_3 = IDL.Variant({
    'Ok' : TransferResponse,
    'Err' : WalletError,
  });
  const ConfiguredExplorerResponse = IDL.Record({
    'token_url_template' : IDL.Opt(IDL.Text),
    'network' : IDL.Text,
    'address_url_template' : IDL.Text,
  });
  const ConfiguredRpcResponse = IDL.Record({
    'network' : IDL.Text,
    'rpc_url' : IDL.Text,
  });
  const Result_4 = IDL.Variant({ 'Ok' : IDL.Null, 'Err' : WalletError });
  const RemoveConfiguredRpcRequest = IDL.Record({ 'network' : IDL.Text });
  const Result_5 = IDL.Variant({ 'Ok' : IDL.Bool, 'Err' : WalletError });
  const Result_6 = IDL.Variant({
    'Ok' : IDL.Opt(IDL.Principal),
    'Err' : WalletError,
  });
  const ServiceInfoResponse = IDL.Record({
    'owner' : IDL.Opt(IDL.Principal),
    'note' : IDL.Opt(IDL.Text),
    'version' : IDL.Text,
    'caller' : IDL.Principal,
    'paused' : IDL.Bool,
  });
  const Result_7 = IDL.Variant({
    'Ok' : ConfiguredRpcResponse,
    'Err' : WalletError,
  });
  const NetworkModuleStatus = IDL.Record({
    'note' : IDL.Opt(IDL.Text),
    'network' : IDL.Text,
    'balance_ready' : IDL.Bool,
    'transfer_ready' : IDL.Bool,
  });
  const WalletNetworkInfoResponse = IDL.Record({
    'id' : IDL.Text,
    'default_rpc_url' : IDL.Opt(IDL.Text),
    'primary_symbol' : IDL.Text,
    'supports_balance' : IDL.Bool,
    'shared_address_group' : IDL.Text,
    'supports_send' : IDL.Bool,
    'address_family' : IDL.Text,
  });
  return IDL.Service({
    'add_configured_token' : IDL.Func(
        [AddConfiguredTokenRequest],
        [Result],
        [],
      ),
    'aptos_mainnet_get_balance_apt' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'aptos_mainnet_get_balance_token' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'aptos_mainnet_request_address' : IDL.Func([], [Result_2], []),
    'aptos_mainnet_transfer_apt' : IDL.Func([TransferRequest], [Result_3], []),
    'aptos_mainnet_transfer_token' : IDL.Func(
        [TransferRequest],
        [Result_3],
        [],
      ),
    'arbitrum_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'arbitrum_get_balance_eth' : IDL.Func([BalanceRequest], [Result_1], []),
    'arbitrum_request_address' : IDL.Func([], [Result_2], []),
    'arbitrum_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'arbitrum_transfer_eth' : IDL.Func([TransferRequest], [Result_3], []),
    'avalanche_get_balance_avax' : IDL.Func([BalanceRequest], [Result_1], []),
    'avalanche_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'avalanche_request_address' : IDL.Func([], [Result_2], []),
    'avalanche_transfer_avax' : IDL.Func([TransferRequest], [Result_3], []),
    'avalanche_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'base_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'base_get_balance_eth' : IDL.Func([BalanceRequest], [Result_1], []),
    'base_request_address' : IDL.Func([], [Result_2], []),
    'base_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'base_transfer_eth' : IDL.Func([TransferRequest], [Result_3], []),
    'bitcoin_get_balance_btc' : IDL.Func([BalanceRequest], [Result_1], []),
    'bitcoin_request_address' : IDL.Func([], [Result_2], []),
    'bitcoin_transfer_btc' : IDL.Func([TransferRequest], [Result_3], []),
    'bsc_get_balance_bep20' : IDL.Func([BalanceRequest], [Result_1], []),
    'bsc_get_balance_bnb' : IDL.Func([BalanceRequest], [Result_1], []),
    'bsc_request_address' : IDL.Func([], [Result_2], []),
    'bsc_transfer_bep20' : IDL.Func([TransferRequest], [Result_3], []),
    'bsc_transfer_bnb' : IDL.Func([TransferRequest], [Result_3], []),
    'configured_explorer' : IDL.Func(
        [IDL.Text],
        [IDL.Opt(ConfiguredExplorerResponse)],
        ['query'],
      ),
    'configured_rpcs' : IDL.Func(
        [],
        [IDL.Vec(ConfiguredRpcResponse)],
        ['query'],
      ),
    'configured_tokens' : IDL.Func(
        [IDL.Text],
        [IDL.Vec(ConfiguredTokenResponse)],
        ['query'],
      ),
    'ethereum_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'ethereum_get_balance_eth' : IDL.Func([BalanceRequest], [Result_1], []),
    'ethereum_request_address' : IDL.Func([], [Result_2], []),
    'ethereum_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'ethereum_transfer_eth' : IDL.Func([TransferRequest], [Result_3], []),
    'get_owner' : IDL.Func([], [IDL.Opt(IDL.Principal)], ['query']),
    'internet_computer_get_balance_icp' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        ['composite_query'],
      ),
    'internet_computer_get_balance_icrc' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        ['composite_query'],
      ),
    'internet_computer_request_address' : IDL.Func([], [Result_2], []),
    'internet_computer_transfer_icp' : IDL.Func(
        [TransferRequest],
        [Result_3],
        [],
      ),
    'internet_computer_transfer_icrc' : IDL.Func(
        [TransferRequest],
        [Result_3],
        [],
      ),
    'is_paused' : IDL.Func([], [IDL.Bool], ['query']),
    'near_mainnet_get_balance_near' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'near_mainnet_get_balance_nep141' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'near_mainnet_request_address' : IDL.Func([], [Result_2], []),
    'near_mainnet_transfer_near' : IDL.Func([TransferRequest], [Result_3], []),
    'near_mainnet_transfer_nep141' : IDL.Func(
        [TransferRequest],
        [Result_3],
        [],
      ),
    'okx_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'okx_get_balance_okb' : IDL.Func([BalanceRequest], [Result_1], []),
    'okx_request_address' : IDL.Func([], [Result_2], []),
    'okx_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'okx_transfer_okb' : IDL.Func([TransferRequest], [Result_3], []),
    'optimism_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'optimism_get_balance_eth' : IDL.Func([BalanceRequest], [Result_1], []),
    'optimism_request_address' : IDL.Func([], [Result_2], []),
    'optimism_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'optimism_transfer_eth' : IDL.Func([TransferRequest], [Result_3], []),
    'pause' : IDL.Func([], [Result_4], []),
    'polygon_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'polygon_get_balance_pol' : IDL.Func([BalanceRequest], [Result_1], []),
    'polygon_request_address' : IDL.Func([], [Result_2], []),
    'polygon_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'polygon_transfer_pol' : IDL.Func([TransferRequest], [Result_3], []),
    'remove_configured_rpc' : IDL.Func(
        [RemoveConfiguredRpcRequest],
        [Result_5],
        [],
      ),
    'remove_configured_token' : IDL.Func(
        [AddConfiguredTokenRequest],
        [Result_5],
        [],
      ),
    'rotate_owner' : IDL.Func([IDL.Principal], [Result_6], []),
    'sepolia_get_balance_erc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'sepolia_get_balance_eth' : IDL.Func([BalanceRequest], [Result_1], []),
    'sepolia_request_address' : IDL.Func([], [Result_2], []),
    'sepolia_transfer_erc20' : IDL.Func([TransferRequest], [Result_3], []),
    'sepolia_transfer_eth' : IDL.Func([TransferRequest], [Result_3], []),
    'service_info' : IDL.Func([], [ServiceInfoResponse], ['query']),
    'set_configured_rpc' : IDL.Func([ConfiguredRpcResponse], [Result_7], []),
    'solana_get_balance_sol' : IDL.Func([BalanceRequest], [Result_1], []),
    'solana_get_balance_spl' : IDL.Func([BalanceRequest], [Result_1], []),
    'solana_request_address' : IDL.Func([], [Result_2], []),
    'solana_testnet_get_balance_sol' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'solana_testnet_get_balance_spl' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'solana_testnet_request_address' : IDL.Func([], [Result_2], []),
    'solana_testnet_transfer_sol' : IDL.Func([TransferRequest], [Result_3], []),
    'solana_testnet_transfer_spl' : IDL.Func([TransferRequest], [Result_3], []),
    'solana_transfer_sol' : IDL.Func([TransferRequest], [Result_3], []),
    'solana_transfer_spl' : IDL.Func([TransferRequest], [Result_3], []),
    'sui_mainnet_get_balance_sui' : IDL.Func([BalanceRequest], [Result_1], []),
    'sui_mainnet_get_balance_token' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'sui_mainnet_request_address' : IDL.Func([], [Result_2], []),
    'sui_mainnet_transfer_sui' : IDL.Func([TransferRequest], [Result_3], []),
    'sui_mainnet_transfer_token' : IDL.Func([TransferRequest], [Result_3], []),
    'supported_networks' : IDL.Func(
        [],
        [IDL.Vec(NetworkModuleStatus)],
        ['query'],
      ),
    'ton_mainnet_get_balance_jetton' : IDL.Func(
        [BalanceRequest],
        [Result_1],
        [],
      ),
    'ton_mainnet_get_balance_ton' : IDL.Func([BalanceRequest], [Result_1], []),
    'ton_mainnet_request_address' : IDL.Func([], [Result_2], []),
    'ton_mainnet_transfer_jetton' : IDL.Func([TransferRequest], [Result_3], []),
    'ton_mainnet_transfer_ton' : IDL.Func([TransferRequest], [Result_3], []),
    'tron_get_balance_trc20' : IDL.Func([BalanceRequest], [Result_1], []),
    'tron_get_balance_trx' : IDL.Func([BalanceRequest], [Result_1], []),
    'tron_request_address' : IDL.Func([], [Result_2], []),
    'tron_transfer_trc20' : IDL.Func([TransferRequest], [Result_3], []),
    'tron_transfer_trx' : IDL.Func([TransferRequest], [Result_3], []),
    'unpause' : IDL.Func([], [Result_4], []),
    'wallet_networks' : IDL.Func(
        [],
        [IDL.Vec(WalletNetworkInfoResponse)],
        ['query'],
      ),
    'whoami' : IDL.Func([], [IDL.Principal], ['query']),
  });
};
export const init = ({ IDL }) => { return []; };
