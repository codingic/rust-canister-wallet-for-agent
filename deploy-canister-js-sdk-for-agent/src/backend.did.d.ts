import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface AddConfiguredTokenRequest {
  'token_address' : string,
  'network' : string,
}
export interface AddressResponse {
  'network' : string,
  'message' : [] | [string],
  'address' : string,
  'key_name' : string,
  'public_key_hex' : string,
}
export interface BalanceRequest { 'token' : [] | [string], 'account' : string }
export interface BalanceResponse {
  'decimals' : [] | [number],
  'token' : [] | [string],
  'pending' : boolean,
  'network' : string,
  'block_ref' : [] | [string],
  'message' : [] | [string],
  'account' : string,
  'amount' : [] | [string],
}
export interface ConfiguredExplorerResponse {
  'token_url_template' : [] | [string],
  'network' : string,
  'address_url_template' : string,
}
export interface ConfiguredRpcResponse {
  'network' : string,
  'rpc_url' : string,
}
export interface ConfiguredTokenResponse {
  'decimals' : bigint,
  'name' : string,
  'token_address' : string,
  'network' : string,
  'symbol' : string,
}
export interface NetworkModuleStatus {
  'note' : [] | [string],
  'network' : string,
  'balance_ready' : boolean,
  'transfer_ready' : boolean,
}
export interface RemoveConfiguredRpcRequest { 'network' : string }
export type Result = { 'Ok' : ConfiguredTokenResponse } |
  { 'Err' : WalletError };
export type Result_1 = { 'Ok' : BalanceResponse } |
  { 'Err' : WalletError };
export type Result_2 = { 'Ok' : AddressResponse } |
  { 'Err' : WalletError };
export type Result_3 = { 'Ok' : TransferResponse } |
  { 'Err' : WalletError };
export type Result_4 = { 'Ok' : null } |
  { 'Err' : WalletError };
export type Result_5 = { 'Ok' : boolean } |
  { 'Err' : WalletError };
export type Result_6 = { 'Ok' : [] | [Principal] } |
  { 'Err' : WalletError };
export type Result_7 = { 'Ok' : ConfiguredRpcResponse } |
  { 'Err' : WalletError };
export interface ServiceInfoResponse {
  'owner' : [] | [Principal],
  'note' : [] | [string],
  'version' : string,
  'caller' : Principal,
  'paused' : boolean,
}
export interface TransferRequest {
  'to' : string,
  'token' : [] | [string],
  'metadata' : Array<[string, string]>,
  'from' : [] | [string],
  'memo' : [] | [string],
  'nonce' : [] | [string],
  'amount' : string,
}
export interface TransferResponse {
  'tx_id' : [] | [string],
  'network' : string,
  'message' : string,
  'accepted' : boolean,
}
export type WalletError = { 'Internal' : string } |
  { 'Paused' : null } |
  { 'InvalidInput' : string } |
  { 'Forbidden' : null } |
  { 'Unimplemented' : { 'network' : string, 'operation' : string } };
export interface WalletNetworkInfoResponse {
  'id' : string,
  'default_rpc_url' : [] | [string],
  'primary_symbol' : string,
  'supports_balance' : boolean,
  'shared_address_group' : string,
  'supports_send' : boolean,
  'address_family' : string,
}
export interface _SERVICE {
  'add_configured_token' : ActorMethod<[AddConfiguredTokenRequest], Result>,
  'aptos_mainnet_get_balance_apt' : ActorMethod<[BalanceRequest], Result_1>,
  'aptos_mainnet_get_balance_token' : ActorMethod<[BalanceRequest], Result_1>,
  'aptos_mainnet_request_address' : ActorMethod<[], Result_2>,
  'aptos_mainnet_transfer_apt' : ActorMethod<[TransferRequest], Result_3>,
  'aptos_mainnet_transfer_token' : ActorMethod<[TransferRequest], Result_3>,
  'arbitrum_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'arbitrum_get_balance_eth' : ActorMethod<[BalanceRequest], Result_1>,
  'arbitrum_request_address' : ActorMethod<[], Result_2>,
  'arbitrum_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'arbitrum_transfer_eth' : ActorMethod<[TransferRequest], Result_3>,
  'avalanche_get_balance_avax' : ActorMethod<[BalanceRequest], Result_1>,
  'avalanche_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'avalanche_request_address' : ActorMethod<[], Result_2>,
  'avalanche_transfer_avax' : ActorMethod<[TransferRequest], Result_3>,
  'avalanche_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'base_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'base_get_balance_eth' : ActorMethod<[BalanceRequest], Result_1>,
  'base_request_address' : ActorMethod<[], Result_2>,
  'base_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'base_transfer_eth' : ActorMethod<[TransferRequest], Result_3>,
  'bitcoin_get_balance_btc' : ActorMethod<[BalanceRequest], Result_1>,
  'bitcoin_request_address' : ActorMethod<[], Result_2>,
  'bitcoin_transfer_btc' : ActorMethod<[TransferRequest], Result_3>,
  'bsc_get_balance_bep20' : ActorMethod<[BalanceRequest], Result_1>,
  'bsc_get_balance_bnb' : ActorMethod<[BalanceRequest], Result_1>,
  'bsc_request_address' : ActorMethod<[], Result_2>,
  'bsc_transfer_bep20' : ActorMethod<[TransferRequest], Result_3>,
  'bsc_transfer_bnb' : ActorMethod<[TransferRequest], Result_3>,
  'configured_explorer' : ActorMethod<
    [string],
    [] | [ConfiguredExplorerResponse]
  >,
  'configured_rpcs' : ActorMethod<[], Array<ConfiguredRpcResponse>>,
  'configured_tokens' : ActorMethod<[string], Array<ConfiguredTokenResponse>>,
  'ethereum_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'ethereum_get_balance_eth' : ActorMethod<[BalanceRequest], Result_1>,
  'ethereum_request_address' : ActorMethod<[], Result_2>,
  'ethereum_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'ethereum_transfer_eth' : ActorMethod<[TransferRequest], Result_3>,
  'get_owner' : ActorMethod<[], [] | [Principal]>,
  'internet_computer_get_balance_icp' : ActorMethod<[BalanceRequest], Result_1>,
  'internet_computer_get_balance_icrc' : ActorMethod<
    [BalanceRequest],
    Result_1
  >,
  'internet_computer_request_address' : ActorMethod<[], Result_2>,
  'internet_computer_transfer_icp' : ActorMethod<[TransferRequest], Result_3>,
  'internet_computer_transfer_icrc' : ActorMethod<[TransferRequest], Result_3>,
  'is_paused' : ActorMethod<[], boolean>,
  'near_mainnet_get_balance_near' : ActorMethod<[BalanceRequest], Result_1>,
  'near_mainnet_get_balance_nep141' : ActorMethod<[BalanceRequest], Result_1>,
  'near_mainnet_request_address' : ActorMethod<[], Result_2>,
  'near_mainnet_transfer_near' : ActorMethod<[TransferRequest], Result_3>,
  'near_mainnet_transfer_nep141' : ActorMethod<[TransferRequest], Result_3>,
  'okx_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'okx_get_balance_okb' : ActorMethod<[BalanceRequest], Result_1>,
  'okx_request_address' : ActorMethod<[], Result_2>,
  'okx_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'okx_transfer_okb' : ActorMethod<[TransferRequest], Result_3>,
  'optimism_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'optimism_get_balance_eth' : ActorMethod<[BalanceRequest], Result_1>,
  'optimism_request_address' : ActorMethod<[], Result_2>,
  'optimism_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'optimism_transfer_eth' : ActorMethod<[TransferRequest], Result_3>,
  'pause' : ActorMethod<[], Result_4>,
  'polygon_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'polygon_get_balance_pol' : ActorMethod<[BalanceRequest], Result_1>,
  'polygon_request_address' : ActorMethod<[], Result_2>,
  'polygon_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'polygon_transfer_pol' : ActorMethod<[TransferRequest], Result_3>,
  'remove_configured_rpc' : ActorMethod<[RemoveConfiguredRpcRequest], Result_5>,
  'remove_configured_token' : ActorMethod<
    [AddConfiguredTokenRequest],
    Result_5
  >,
  'rotate_owner' : ActorMethod<[Principal], Result_6>,
  'sepolia_get_balance_erc20' : ActorMethod<[BalanceRequest], Result_1>,
  'sepolia_get_balance_eth' : ActorMethod<[BalanceRequest], Result_1>,
  'sepolia_request_address' : ActorMethod<[], Result_2>,
  'sepolia_transfer_erc20' : ActorMethod<[TransferRequest], Result_3>,
  'sepolia_transfer_eth' : ActorMethod<[TransferRequest], Result_3>,
  'service_info' : ActorMethod<[], ServiceInfoResponse>,
  'set_configured_rpc' : ActorMethod<[ConfiguredRpcResponse], Result_7>,
  'solana_get_balance_sol' : ActorMethod<[BalanceRequest], Result_1>,
  'solana_get_balance_spl' : ActorMethod<[BalanceRequest], Result_1>,
  'solana_request_address' : ActorMethod<[], Result_2>,
  'solana_testnet_get_balance_sol' : ActorMethod<[BalanceRequest], Result_1>,
  'solana_testnet_get_balance_spl' : ActorMethod<[BalanceRequest], Result_1>,
  'solana_testnet_request_address' : ActorMethod<[], Result_2>,
  'solana_testnet_transfer_sol' : ActorMethod<[TransferRequest], Result_3>,
  'solana_testnet_transfer_spl' : ActorMethod<[TransferRequest], Result_3>,
  'solana_transfer_sol' : ActorMethod<[TransferRequest], Result_3>,
  'solana_transfer_spl' : ActorMethod<[TransferRequest], Result_3>,
  'sui_mainnet_get_balance_sui' : ActorMethod<[BalanceRequest], Result_1>,
  'sui_mainnet_get_balance_token' : ActorMethod<[BalanceRequest], Result_1>,
  'sui_mainnet_request_address' : ActorMethod<[], Result_2>,
  'sui_mainnet_transfer_sui' : ActorMethod<[TransferRequest], Result_3>,
  'sui_mainnet_transfer_token' : ActorMethod<[TransferRequest], Result_3>,
  'supported_networks' : ActorMethod<[], Array<NetworkModuleStatus>>,
  'ton_mainnet_get_balance_jetton' : ActorMethod<[BalanceRequest], Result_1>,
  'ton_mainnet_get_balance_ton' : ActorMethod<[BalanceRequest], Result_1>,
  'ton_mainnet_request_address' : ActorMethod<[], Result_2>,
  'ton_mainnet_transfer_jetton' : ActorMethod<[TransferRequest], Result_3>,
  'ton_mainnet_transfer_ton' : ActorMethod<[TransferRequest], Result_3>,
  'tron_get_balance_trc20' : ActorMethod<[BalanceRequest], Result_1>,
  'tron_get_balance_trx' : ActorMethod<[BalanceRequest], Result_1>,
  'tron_request_address' : ActorMethod<[], Result_2>,
  'tron_transfer_trc20' : ActorMethod<[TransferRequest], Result_3>,
  'tron_transfer_trx' : ActorMethod<[TransferRequest], Result_3>,
  'unpause' : ActorMethod<[], Result_4>,
  'wallet_networks' : ActorMethod<[], Array<WalletNetworkInfoResponse>>,
  'whoami' : ActorMethod<[], Principal>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
