use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest,
    TransferResponse,
};
use k256::elliptic_curve::bigint::U256;
use k256::elliptic_curve::ops::Reduce;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::schnorr::VerifyingKey as SchnorrVerifyingKey;
use k256::{ProjectivePoint, PublicKey, Scalar};
use sha2::{Digest, Sha256};

const NETWORK_NAME: &str = "btc";

pub async fn request_address(req: AddressRequest) -> WalletResult<AddressResponse> {
    let resolved = addressing::resolve_address_request(NETWORK_NAME, req)?;
    let (public_key, key_name) = addressing::fetch_schnorr_public_key(
        ic_cdk::management_canister::SchnorrAlgorithm::Bip340secp256k1,
        resolved.derivation_path,
    )
    .await?;

    let internal_key = parse_bip340_internal_key(&public_key)?;
    let witness_program = taproot_output_key(&internal_key)?;
    let address = addressing::encode_segwit_v1_bech32m(bitcoin_hrp(), &witness_program)?;

    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address,
        public_key_hex: addressing::hex_encode(&public_key),
        key_name,
        index: resolved.index,
        account_tag: resolved.account_tag,
        message: Some("Derived taproot address from management canister Schnorr public key".into()),
    })
}

pub fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: req.token,
        amount: None,
        decimals: Some(8),
        block_ref: None,
        pending: true,
        message: Some("BTC balance query not implemented yet".to_string()),
    })
}

pub fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: false,
        tx_id: None,
        message: format!(
            "{NETWORK_NAME} transfer scaffold received request; signing/execution not implemented"
        ),
    })
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

fn bitcoin_hrp() -> &'static str {
    if config::app_config::is_dev_mode() {
        "bcrt"
    } else {
        "bc"
    }
}

fn parse_bip340_internal_key(raw: &[u8]) -> WalletResult<[u8; 32]> {
    match raw.len() {
        32 => {
            let mut x_only = [0u8; 32];
            x_only.copy_from_slice(raw);
            // Validate the x-coordinate lifts to a valid even-y secp256k1 point.
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
