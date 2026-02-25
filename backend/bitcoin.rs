use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest, TransferResponse,
};
use candid::Nat;
use ic_cdk::bitcoin_canister::{Outpoint, Utxo};
use ic_cdk::management_canister::{
    self, Bip341, SchnorrAlgorithm, SchnorrAux, SchnorrKeyId, SignWithSchnorrArgs,
};
use k256::elliptic_curve::bigint::U256;
use k256::elliptic_curve::ops::Reduce;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::schnorr::VerifyingKey as SchnorrVerifyingKey;
use k256::{ProjectivePoint, PublicKey, Scalar};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const NETWORK_NAME: &str = types::networks::BITCOIN;
const BTC_DECIMALS: u8 = 8;
const DEFAULT_FEE_RATE_SAT_PER_VB: u64 = 5;
const MIN_CHANGE_SATS: u64 = 330;
const SIGHASH_DEFAULT: u8 = 0x00;
const SEQUENCE_FINAL: u32 = 0xffff_ffff;
const TX_VERSION: u32 = 2;
const TX_LOCKTIME: u32 = 0;
const BECH32_CONST: u32 = 1;
const BECH32M_CONST: u32 = 0x2bc8_30a3;
const BECH32_CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

#[derive(Deserialize)]
struct MempoolAddressStats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
}

#[derive(Deserialize)]
struct MempoolAddressResponse {
    chain_stats: MempoolAddressStats,
    mempool_stats: Option<MempoolAddressStats>,
}

#[derive(Deserialize)]
struct MempoolUtxoStatus {
    confirmed: bool,
    block_height: Option<u32>,
}

#[derive(Deserialize)]
struct MempoolUtxoResponse {
    txid: String,
    vout: u32,
    value: u64,
    status: MempoolUtxoStatus,
}

#[derive(Clone, Debug)]
struct WalletBtcKey {
    address: String,
    key_name: String,
    internal_key_x_only: [u8; 32],
    taproot_witness_program: [u8; 32],
}

#[derive(Clone, Debug)]
struct TxInputTemplate {
    utxo: Utxo,
    sequence: u32,
}

#[derive(Clone, Debug)]
struct TxOutputTemplate {
    value: u64,
    script_pubkey: Vec<u8>,
}

pub async fn request_address() -> WalletResult<AddressResponse> {
    let wallet_key = derive_wallet_key().await?;

    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: wallet_key.address,
        public_key_hex: addressing::hex_encode(&wallet_key.internal_key_x_only),
        key_name: wallet_key.key_name,
        message: Some("Derived taproot address from management canister Schnorr public key".into()),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    if req.token.as_deref().is_some_and(|t| !t.trim().is_empty()) {
        return Err(WalletError::invalid_input(
            "btc_get_balance_btc does not accept token parameter",
        ));
    }

    let address = req.account.trim().to_string();
    let mempool_addr = fetch_address_stats(&address).await?;
    let confirmed_sats = mempool_addr
        .chain_stats
        .funded_txo_sum
        .saturating_sub(mempool_addr.chain_stats.spent_txo_sum);
    let pending_delta = mempool_addr
        .mempool_stats
        .map(|s| s.funded_txo_sum.saturating_sub(s.spent_txo_sum))
        .unwrap_or(0);
    let sats = confirmed_sats.saturating_add(pending_delta);

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: address,
        token: None,
        amount: Some(format_sats_btc(sats)),
        decimals: Some(BTC_DECIMALS),
        block_ref: None,
        pending: false,
        message: Some("BTC RPC address stats (confirmed + mempool delta)".to_string()),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    if req.token.as_deref().is_some_and(|t| !t.trim().is_empty()) {
        return Err(WalletError::invalid_input(
            "btc_transfer_btc does not accept token parameter",
        ));
    }

    let wallet_key = derive_wallet_key().await?;
    if let Some(from) = req.from.as_deref() {
        let from = from.trim();
        if !from.is_empty() && !from.eq_ignore_ascii_case(&wallet_key.address) {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed BTC address",
            ));
        }
    }

    let amount_sats = parse_decimal_btc_to_sats(req.amount.trim())?;
    if amount_sats == 0 {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let to_address = req.to.trim().to_lowercase();
    let to_script = script_pubkey_from_btc_address(&to_address, &expected_hrp())?;
    let change_script = script_pubkey_p2tr(&wallet_key.taproot_witness_program);
    let source_script = change_script.clone();

    let utxos = fetch_all_utxos(&wallet_key.address).await?;
    if utxos.is_empty() {
        return Err(WalletError::Internal("no BTC UTXOs available".into()));
    }

    let fee_rate = fetch_fee_rate_sat_per_vb()
        .await
        .unwrap_or(DEFAULT_FEE_RATE_SAT_PER_VB);

    let plan = build_spend_plan(&utxos, amount_sats, &to_script, &change_script, fee_rate)?;

    let mut witnesses: Vec<Vec<Vec<u8>>> = Vec::with_capacity(plan.inputs.len());
    for _ in 0..plan.inputs.len() {
        witnesses.push(Vec::new());
    }

    for input_index in 0..plan.inputs.len() {
        let sighash = taproot_key_spend_sighash(
            TX_VERSION,
            TX_LOCKTIME,
            &plan.inputs,
            &plan.outputs,
            input_index,
            &source_script,
        )?;
        let sig = sign_taproot_keypath_sighash(&sighash, &wallet_key.key_name).await?;
        witnesses[input_index] = vec![sig];
    }

    let tx_bytes = serialize_tx(&plan.inputs, &plan.outputs, &witnesses, true);
    let raw_tx_hex = addressing::hex_encode(&tx_bytes);
    let txid_from_rpc = broadcast_raw_transaction(&raw_tx_hex).await?;

    let txid = if txid_from_rpc.trim().is_empty() {
        txid_hex(&tx_bytes, &plan.inputs, &plan.outputs, &witnesses)
    } else {
        txid_from_rpc
    };

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: Some(txid),
        message: format!(
            "btc rpc send accepted (fee={} sats, fee_rate={} sat/vB)",
            plan.fee_sats, plan.fee_rate_sat_per_vb
        ),
    })
}

#[derive(Clone, Debug)]
struct SpendPlan {
    inputs: Vec<TxInputTemplate>,
    outputs: Vec<TxOutputTemplate>,
    fee_sats: u64,
    fee_rate_sat_per_vb: u64,
}

fn build_spend_plan(
    utxos: &[Utxo],
    amount_sats: u64,
    to_script: &[u8],
    change_script: &[u8],
    fee_rate_sat_per_vb: u64,
) -> WalletResult<SpendPlan> {
    let mut selected: Vec<Utxo> = Vec::new();
    let mut total_in: u64 = 0;
    let fee_rate_sat_per_vb = fee_rate_sat_per_vb.max(1);

    let mut sorted_utxos = utxos.to_vec();
    sorted_utxos.sort_by_key(|u| u.value);

    for utxo in sorted_utxos {
        total_in = total_in
            .checked_add(utxo.value)
            .ok_or_else(|| WalletError::Internal("BTC input sum overflow".into()))?;
        selected.push(utxo);

        let inputs = selected
            .iter()
            .cloned()
            .map(|u| TxInputTemplate {
                utxo: u,
                sequence: SEQUENCE_FINAL,
            })
            .collect::<Vec<_>>();

        let one_output = vec![TxOutputTemplate {
            value: amount_sats,
            script_pubkey: to_script.to_vec(),
        }];
        let fee_no_change =
            estimate_signed_tx_vbytes(inputs.len(), &one_output) as u64 * fee_rate_sat_per_vb;
        let needed_no_change = amount_sats
            .checked_add(fee_no_change)
            .ok_or_else(|| WalletError::Internal("BTC amount overflow".into()))?;

        if total_in < needed_no_change {
            continue;
        }

        let fee_with_change = estimate_signed_tx_vbytes(
            inputs.len(),
            &[
                TxOutputTemplate {
                    value: amount_sats,
                    script_pubkey: to_script.to_vec(),
                },
                TxOutputTemplate {
                    value: 0,
                    script_pubkey: change_script.to_vec(),
                },
            ],
        ) as u64
            * fee_rate_sat_per_vb;

        let needed_with_change = amount_sats
            .checked_add(fee_with_change)
            .ok_or_else(|| WalletError::Internal("BTC amount overflow".into()))?;

        if total_in >= needed_with_change {
            let change = total_in - needed_with_change;
            if change >= MIN_CHANGE_SATS {
                let outputs = vec![
                    TxOutputTemplate {
                        value: amount_sats,
                        script_pubkey: to_script.to_vec(),
                    },
                    TxOutputTemplate {
                        value: change,
                        script_pubkey: change_script.to_vec(),
                    },
                ];
                return Ok(SpendPlan {
                    inputs,
                    outputs,
                    fee_sats: fee_with_change,
                    fee_rate_sat_per_vb,
                });
            }
        }

        let fee = fee_no_change;
        if total_in >= amount_sats + fee {
            let outputs = vec![TxOutputTemplate {
                value: amount_sats,
                script_pubkey: to_script.to_vec(),
            }];
            return Ok(SpendPlan {
                inputs,
                outputs,
                fee_sats: fee,
                fee_rate_sat_per_vb,
            });
        }
    }

    Err(WalletError::Internal(
        "insufficient BTC funds (including fee)".into(),
    ))
}

async fn derive_wallet_key() -> WalletResult<WalletBtcKey> {
    let (public_key, key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Bip340secp256k1).await?;
    let internal_key = parse_bip340_internal_key(&public_key)?;
    let witness_program = taproot_output_key(&internal_key)?;
    let address = addressing::encode_segwit_v1_bech32m(bitcoin_hrp(), &witness_program)?;

    Ok(WalletBtcKey {
        address,
        key_name,
        internal_key_x_only: internal_key,
        taproot_witness_program: witness_program,
    })
}

async fn fetch_all_utxos(address: &str) -> WalletResult<Vec<Utxo>> {
    let rows: Vec<MempoolUtxoResponse> =
        btc_rpc_get_json(&format!("/address/{address}/utxo")).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let txid_bytes = parse_txid_hex_to_bytes(&row.txid)?;
        out.push(Utxo {
            outpoint: Outpoint {
                txid: txid_bytes,
                vout: row.vout,
            },
            value: row.value,
            height: if row.status.confirmed {
                row.status.block_height.unwrap_or(0)
            } else {
                0
            },
        });
    }
    Ok(out)
}

async fn fetch_fee_rate_sat_per_vb() -> WalletResult<u64> {
    let fees: Value = btc_rpc_get_json("/fee-estimates").await?;
    let candidates = ["3", "2", "6", "1"];
    for key in candidates {
        if let Some(v) = fees.get(key).and_then(Value::as_f64) {
            let sat_vb = v.ceil() as u64;
            return Ok(sat_vb.max(1));
        }
    }
    Ok(DEFAULT_FEE_RATE_SAT_PER_VB)
}

async fn sign_taproot_keypath_sighash(
    sighash32: &[u8; 32],
    key_name: &str,
) -> WalletResult<Vec<u8>> {
    let result = management_canister::sign_with_schnorr(&SignWithSchnorrArgs {
        message: sighash32.to_vec(),
        derivation_path: vec![],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Bip340secp256k1,
            name: key_name.to_string(),
        },
        aux: Some(SchnorrAux::Bip341(Bip341 {
            merkle_root_hash: vec![],
        })),
    })
    .await
    .map_err(|err| WalletError::Internal(format!("sign_with_schnorr failed: {err}")))?;

    if result.signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected taproot signature length: {}",
            result.signature.len()
        )));
    }
    Ok(result.signature)
}

fn expected_hrp() -> String {
    bitcoin_hrp().to_string()
}

fn bitcoin_hrp() -> &'static str {
    "bc"
}

async fn fetch_address_stats(address: &str) -> WalletResult<MempoolAddressResponse> {
    btc_rpc_get_json(&format!("/address/{address}")).await
}

async fn broadcast_raw_transaction(raw_tx_hex: &str) -> WalletResult<String> {
    let body = raw_tx_hex.trim().as_bytes().to_vec();
    btc_rpc_post_text("/tx", body, "text/plain").await
}

fn bitcoin_rpc_base_url() -> WalletResult<String> {
    config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|err| WalletError::Internal(format!("bitcoin rpc url resolve failed: {err}")))
}

async fn btc_rpc_get_json<T>(path: &str) -> WalletResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let http_res = crate::outcall::get_json(
        format!("{}{}", bitcoin_rpc_base_url()?, path),
        512 * 1024,
        "btc rpc",
    )
    .await?;

    if http_res.status != Nat::from(200u16) {
        let body_text = String::from_utf8_lossy(&http_res.body);
        let snippet: String = body_text.chars().take(240).collect();
        return Err(WalletError::Internal(format!(
            "btc rpc http status {}: {}",
            http_res.status, snippet
        )));
    }

    serde_json::from_slice::<T>(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("btc rpc parse response failed: {err}")))
}

async fn btc_rpc_post_text(path: &str, body: Vec<u8>, content_type: &str) -> WalletResult<String> {
    let http_res = crate::outcall::post_text(
        format!("{}{}", bitcoin_rpc_base_url()?, path),
        body,
        content_type,
        "text/plain",
        64 * 1024,
        "btc rpc",
    )
    .await?;
    if http_res.status != Nat::from(200u16) {
        let body_text = String::from_utf8_lossy(&http_res.body);
        let snippet: String = body_text.chars().take(240).collect();
        return Err(WalletError::Internal(format!(
            "btc rpc post status {}: {}",
            http_res.status, snippet
        )));
    }
    let text = String::from_utf8(http_res.body)
        .map_err(|err| WalletError::Internal(format!("btc rpc response is not utf8: {err}")))?;
    Ok(text.trim().to_string())
}

fn parse_txid_hex_to_bytes(txid_hex: &str) -> WalletResult<Vec<u8>> {
    let hex = txid_hex.trim();
    if hex.len() != 64 {
        return Err(WalletError::Internal(format!(
            "btc rpc utxo txid length invalid: {}",
            hex.len()
        )));
    }
    let mut out = Vec::with_capacity(32);
    let bytes = hex.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = decode_hex_nibble(bytes[i])?;
        let lo = decode_hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_hex_nibble(b: u8) -> WalletResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(WalletError::Internal("btc rpc returned invalid hex".into())),
    }
}

fn validate_account(account: &str) -> WalletResult<()> {
    if account.trim().is_empty() {
        return Err(WalletError::invalid_input("account is required"));
    }
    Ok(())
}

fn validate_transfer(req: &TransferRequest) -> WalletResult<()> {
    if req.to.trim().is_empty() {
        return Err(WalletError::invalid_input("to is required"));
    }
    if req.amount.trim().is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    Ok(())
}

fn parse_decimal_btc_to_sats(value: &str) -> WalletResult<u64> {
    let value = value.trim();
    if value.is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    if value.starts_with('-') {
        return Err(WalletError::invalid_input("amount must be positive"));
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let frac = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::invalid_input("amount format is invalid"));
    }
    let whole = normalize_numeric_separators(whole);
    if !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err(WalletError::invalid_input("amount must be decimal"));
    }
    let mut sats: u128 = if whole.is_empty() {
        0
    } else {
        whole
            .parse::<u128>()
            .map_err(|_| WalletError::invalid_input("amount parse failed"))?
    };
    sats = sats
        .checked_mul(100_000_000u128)
        .ok_or_else(|| WalletError::invalid_input("amount too large"))?;

    if let Some(frac) = frac {
        let mut frac = normalize_numeric_separators(frac);
        if !frac.chars().all(|c| c.is_ascii_digit()) {
            return Err(WalletError::invalid_input("amount must be decimal"));
        }
        if frac.len() > 8 {
            return Err(WalletError::invalid_input("too many decimal places"));
        }
        while frac.len() < 8 {
            frac.push('0');
        }
        if !frac.is_empty() {
            sats = sats
                .checked_add(
                    frac.parse::<u128>()
                        .map_err(|_| WalletError::invalid_input("amount parse failed"))?,
                )
                .ok_or_else(|| WalletError::invalid_input("amount too large"))?;
        }
    }
    u64::try_from(sats).map_err(|_| WalletError::invalid_input("amount too large"))
}

fn format_sats_btc(sats: u64) -> String {
    let whole = sats / 100_000_000;
    let frac = sats % 100_000_000;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_str = format!("{frac:08}");
    while frac_str.ends_with('0') {
        frac_str.pop();
    }
    format!("{whole}.{frac_str}")
}

fn normalize_numeric_separators(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|c| *c != '_' && *c != ',')
        .collect()
}

fn parse_bip340_internal_key(raw: &[u8]) -> WalletResult<[u8; 32]> {
    match raw.len() {
        32 => {
            let mut x_only = [0u8; 32];
            x_only.copy_from_slice(raw);
            SchnorrVerifyingKey::from_bytes(&x_only)
                .map_err(|_| WalletError::Internal("invalid BIP340 x-only public key".into()))?;
            Ok(x_only)
        }
        33 => {
            let pk = PublicKey::from_sec1_bytes(raw).map_err(|err| {
                WalletError::Internal(format!("invalid BTC secp256k1 key: {err}"))
            })?;
            let compressed = pk.to_encoded_point(true);
            let compressed_bytes = compressed.as_bytes();
            if compressed_bytes.len() != 33 {
                return Err(WalletError::Internal(
                    "unexpected compressed secp256k1 key length".into(),
                ));
            }
            let mut x_only = [0u8; 32];
            x_only.copy_from_slice(&compressed_bytes[1..33]);
            Ok(x_only)
        }
        n => Err(WalletError::Internal(format!(
            "unexpected BTC public key length: {n}"
        ))),
    }
}

fn taproot_output_key(internal_key_x_only: &[u8; 32]) -> WalletResult<[u8; 32]> {
    let internal_vk = SchnorrVerifyingKey::from_bytes(internal_key_x_only)
        .map_err(|_| WalletError::Internal("invalid taproot internal key".into()))?;
    let internal_pk: PublicKey = internal_vk.into();
    let internal_point = ProjectivePoint::from(*internal_pk.as_affine());

    let tweak_hash = tagged_hash_sha256(b"TapTweak", internal_key_x_only);
    let tweak_scalar = scalar_from_hash(&tweak_hash)?;
    let output_point = internal_point + (ProjectivePoint::GENERATOR * tweak_scalar);
    let output_affine = k256::AffinePoint::from(output_point);
    let output_pk = PublicKey::from_affine(output_affine)
        .map_err(|_| WalletError::Internal("taproot output key is invalid".into()))?;
    let output_compressed = output_pk.to_encoded_point(true);
    let output_bytes = output_compressed.as_bytes();
    if output_bytes.len() != 33 {
        return Err(WalletError::Internal(
            "unexpected taproot compressed key length".into(),
        ));
    }

    let mut witness_program = [0u8; 32];
    witness_program.copy_from_slice(&output_bytes[1..33]);
    Ok(witness_program)
}

fn scalar_from_hash(hash32: &[u8; 32]) -> WalletResult<Scalar> {
    let mut field_bytes = k256::FieldBytes::default();
    field_bytes.copy_from_slice(hash32);
    let scalar = <Scalar as Reduce<U256>>::reduce_bytes(&field_bytes);
    if bool::from(scalar.is_zero()) {
        return Err(WalletError::Internal(
            "taproot tweak reduced to zero scalar".into(),
        ));
    }
    Ok(scalar)
}

fn tagged_hash_sha256(tag: &[u8], msg: &[u8]) -> [u8; 32] {
    let tag_hash = Sha256::digest(tag);
    let digest = Sha256::new()
        .chain_update(tag_hash)
        .chain_update(tag_hash)
        .chain_update(msg)
        .finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn taproot_key_spend_sighash(
    version: u32,
    lock_time: u32,
    inputs: &[TxInputTemplate],
    outputs: &[TxOutputTemplate],
    input_index: usize,
    source_script_pubkey: &[u8],
) -> WalletResult<[u8; 32]> {
    if input_index >= inputs.len() {
        return Err(WalletError::Internal(
            "taproot sighash input index out of range".into(),
        ));
    }

    let mut prevouts_ser = Vec::new();
    let mut amounts_ser = Vec::new();
    let mut scriptpubkeys_ser = Vec::new();
    let mut sequences_ser = Vec::new();
    for input in inputs {
        serialize_outpoint_into(&input.utxo, &mut prevouts_ser);
        amounts_ser.extend_from_slice(&input.utxo.value.to_le_bytes());
        write_compact_size_into(source_script_pubkey.len() as u64, &mut scriptpubkeys_ser);
        scriptpubkeys_ser.extend_from_slice(source_script_pubkey);
        sequences_ser.extend_from_slice(&input.sequence.to_le_bytes());
    }
    let mut outputs_ser = Vec::new();
    for output in outputs {
        serialize_output_into(output, &mut outputs_ser);
    }

    let hash_prevouts = sha256_once(&prevouts_ser);
    let hash_amounts = sha256_once(&amounts_ser);
    let hash_scriptpubkeys = sha256_once(&scriptpubkeys_ser);
    let hash_sequences = sha256_once(&sequences_ser);
    let hash_outputs = sha256_once(&outputs_ser);

    let mut msg = Vec::with_capacity(1 + 1 + 4 + 4 + 32 * 5 + 1 + 4);
    msg.push(0x00); // epoch
    msg.push(SIGHASH_DEFAULT);
    msg.extend_from_slice(&version.to_le_bytes());
    msg.extend_from_slice(&lock_time.to_le_bytes());
    msg.extend_from_slice(&hash_prevouts);
    msg.extend_from_slice(&hash_amounts);
    msg.extend_from_slice(&hash_scriptpubkeys);
    msg.extend_from_slice(&hash_sequences);
    msg.extend_from_slice(&hash_outputs);
    msg.push(0x00); // spend_type = key path, no annex
    msg.extend_from_slice(&(input_index as u32).to_le_bytes());

    Ok(tagged_hash_sha256(b"TapSighash", &msg))
}

fn estimate_signed_tx_vbytes(input_count: usize, outputs: &[TxOutputTemplate]) -> usize {
    let non_witness_len = serialized_tx_len_no_witness(input_count, outputs);
    let witness_len = 2 + input_count * (1 + 1 + 64); // marker+flag + per-input [stack_count=1][push=64][sig]
    let weight = non_witness_len * 4 + witness_len;
    weight.div_ceil(4)
}

fn serialized_tx_len_no_witness(input_count: usize, outputs: &[TxOutputTemplate]) -> usize {
    4 + compact_size_len(input_count as u64)
        + input_count * 41
        + compact_size_len(outputs.len() as u64)
        + outputs.iter().map(serialized_output_len).sum::<usize>()
        + 4
}

fn serialized_output_len(output: &TxOutputTemplate) -> usize {
    8 + compact_size_len(output.script_pubkey.len() as u64) + output.script_pubkey.len()
}

fn serialize_tx(
    inputs: &[TxInputTemplate],
    outputs: &[TxOutputTemplate],
    witnesses: &[Vec<Vec<u8>>],
    include_witness: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&TX_VERSION.to_le_bytes());
    if include_witness {
        out.push(0x00);
        out.push(0x01);
    }
    write_compact_size_into(inputs.len() as u64, &mut out);
    for input in inputs {
        serialize_outpoint_into(&input.utxo, &mut out);
        out.push(0x00); // scriptSig length = 0
        out.extend_from_slice(&input.sequence.to_le_bytes());
    }
    write_compact_size_into(outputs.len() as u64, &mut out);
    for output in outputs {
        serialize_output_into(output, &mut out);
    }
    if include_witness {
        for witness in witnesses {
            write_compact_size_into(witness.len() as u64, &mut out);
            for item in witness {
                write_compact_size_into(item.len() as u64, &mut out);
                out.extend_from_slice(item);
            }
        }
    }
    out.extend_from_slice(&TX_LOCKTIME.to_le_bytes());
    out
}

fn txid_hex(
    _tx_with_witness: &[u8],
    inputs: &[TxInputTemplate],
    outputs: &[TxOutputTemplate],
    witnesses: &[Vec<Vec<u8>>],
) -> String {
    let legacy_ser = serialize_tx(inputs, outputs, witnesses, false);
    let mut txid = double_sha256(&legacy_ser);
    txid.reverse();
    addressing::hex_encode(&txid)
}

fn serialize_outpoint_into(utxo: &Utxo, out: &mut Vec<u8>) {
    let mut txid = utxo.outpoint.txid.clone();
    txid.reverse(); // IC returns txid bytes in txid order; outpoint serialization is little-endian.
    out.extend_from_slice(&txid);
    out.extend_from_slice(&utxo.outpoint.vout.to_le_bytes());
}

fn serialize_output_into(output: &TxOutputTemplate, out: &mut Vec<u8>) {
    out.extend_from_slice(&output.value.to_le_bytes());
    write_compact_size_into(output.script_pubkey.len() as u64, out);
    out.extend_from_slice(&output.script_pubkey);
}

fn script_pubkey_p2tr(witness_program: &[u8; 32]) -> Vec<u8> {
    let mut script = Vec::with_capacity(34);
    script.push(0x51); // OP_1
    script.push(0x20); // push 32
    script.extend_from_slice(witness_program);
    script
}

fn script_pubkey_from_btc_address(address: &str, expected_hrp: &str) -> WalletResult<Vec<u8>> {
    let decoded = decode_segwit_address(address, expected_hrp)?;
    let mut script = Vec::with_capacity(2 + decoded.program.len());
    let op = match decoded.version {
        0 => 0x00,
        1..=16 => 0x50 + decoded.version,
        _ => return Err(WalletError::invalid_input("unsupported witness version")),
    };
    script.push(op);
    script.push(
        u8::try_from(decoded.program.len())
            .map_err(|_| WalletError::invalid_input("witness program too long"))?,
    );
    script.extend_from_slice(&decoded.program);
    Ok(script)
}

#[derive(Clone, Debug)]
struct DecodedSegwitAddress {
    version: u8,
    program: Vec<u8>,
}

fn decode_segwit_address(address: &str, expected_hrp: &str) -> WalletResult<DecodedSegwitAddress> {
    let addr = address.trim().to_lowercase();
    if addr.is_empty() {
        return Err(WalletError::invalid_input("BTC address is required"));
    }

    let sep_pos = addr
        .rfind('1')
        .ok_or_else(|| WalletError::invalid_input("invalid bech32 address"))?;
    if sep_pos == 0 || sep_pos + 7 > addr.len() {
        return Err(WalletError::invalid_input("invalid bech32 address length"));
    }
    let hrp = &addr[..sep_pos];
    if hrp != expected_hrp {
        return Err(WalletError::invalid_input(format!(
            "BTC address hrp mismatch: expected {expected_hrp}, got {hrp}"
        )));
    }

    let mut data = Vec::with_capacity(addr.len() - sep_pos - 1);
    for ch in addr[sep_pos + 1..].bytes() {
        let idx = BECH32_CHARSET
            .iter()
            .position(|v| *v == ch)
            .ok_or_else(|| WalletError::invalid_input("invalid bech32 character"))?;
        data.push(idx as u8);
    }
    if data.len() < 7 {
        return Err(WalletError::invalid_input("invalid bech32 payload"));
    }

    let checksum_variant = verify_bech32_checksum(hrp, &data)?;
    let payload = &data[..data.len() - 6];
    if payload.is_empty() {
        return Err(WalletError::invalid_input("invalid segwit payload"));
    }
    let version = payload[0];
    if version > 16 {
        return Err(WalletError::invalid_input("unsupported witness version"));
    }
    if version == 0 && checksum_variant != Bech32Variant::Bech32 {
        return Err(WalletError::invalid_input(
            "v0 segwit address must use bech32 checksum",
        ));
    }
    if version != 0 && checksum_variant != Bech32Variant::Bech32m {
        return Err(WalletError::invalid_input(
            "segwit v1+ address must use bech32m checksum",
        ));
    }

    let program = convert_bits_5_to_8(&payload[1..])?;
    if !(2..=40).contains(&program.len()) {
        return Err(WalletError::invalid_input("invalid witness program length"));
    }
    if version == 0 && !(program.len() == 20 || program.len() == 32) {
        return Err(WalletError::invalid_input(
            "v0 witness program length must be 20 or 32",
        ));
    }

    Ok(DecodedSegwitAddress { version, program })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bech32Variant {
    Bech32,
    Bech32m,
}

fn verify_bech32_checksum(hrp: &str, data: &[u8]) -> WalletResult<Bech32Variant> {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    let polymod = bech32_polymod(&values);
    match polymod {
        BECH32_CONST => Ok(Bech32Variant::Bech32),
        BECH32M_CONST => Ok(Bech32Variant::Bech32m),
        _ => Err(WalletError::invalid_input("invalid bech32 checksum")),
    }
}

fn convert_bits_5_to_8(data: &[u8]) -> WalletResult<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::new();
    for value in data {
        if *value >= 32 {
            return Err(WalletError::invalid_input("invalid bech32 data value"));
        }
        acc = (acc << 5) | (*value as u32);
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    if bits > 0 && ((acc << (8 - bits)) & 0xff) != 0 {
        return Err(WalletError::invalid_input("invalid bech32 padding"));
    }
    Ok(out)
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(hrp.len() * 2 + 1);
    for b in hrp.bytes() {
        out.push(b >> 5);
    }
    out.push(0);
    for b in hrp.bytes() {
        out.push(b & 0x1f);
    }
    out
}

fn bech32_polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [
        0x3b6a_57b2,
        0x2650_8e6d,
        0x1ea1_19fa,
        0x3d42_33dd,
        0x2a14_62b3,
    ];
    let mut chk = 1u32;
    for value in values {
        let top = (chk >> 25) as u8;
        chk = ((chk & 0x01ff_ffff) << 5) ^ (*value as u32);
        for (i, g) in GEN.iter().enumerate() {
            if ((top >> i) & 1) != 0 {
                chk ^= *g;
            }
        }
    }
    chk
}

fn compact_size_len(n: u64) -> usize {
    match n {
        0..=252 => 1,
        253..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

fn write_compact_size_into(n: u64, out: &mut Vec<u8>) {
    match n {
        0..=252 => out.push(n as u8),
        253..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(n as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&n.to_le_bytes());
        }
    }
}

fn sha256_once(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn double_sha256(data: &[u8]) -> Vec<u8> {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    second.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_parses_btc_units() {
        assert_eq!(format_sats_btc(3_890_000_000), "38.9");
        assert_eq!(parse_decimal_btc_to_sats("38.9").unwrap(), 3_890_000_000);
        assert_eq!(parse_decimal_btc_to_sats("0.00000001").unwrap(), 1);
    }

    #[test]
    fn builds_segwit_scripts() {
        let witness = [7u8; 32];
        let addr = addressing::encode_segwit_v1_bech32m("bc", &witness).unwrap();
        let script = script_pubkey_from_btc_address(&addr, "bc").unwrap();
        assert_eq!(script.len(), 34);
        assert_eq!(script[0], 0x51);
        assert_eq!(script[1], 0x20);
        assert_eq!(&script[2..], &witness);
    }
}
