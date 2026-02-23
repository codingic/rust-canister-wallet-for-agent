use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::AddressRequest;
use ic_cdk::management_canister::{
    self, EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgs, SchnorrAlgorithm, SchnorrKeyId,
    SchnorrPublicKeyArgs,
};

const BECH32M_CONST: u32 = 0x2bc8_30a3;
const BECH32_CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Clone, Debug)]
pub struct ResolvedAddressRequest {
    pub index: u32,
    pub account_tag: Option<String>,
    pub derivation_path: Vec<Vec<u8>>,
}

pub fn resolve_address_request(
    network: &str,
    req: AddressRequest,
) -> WalletResult<ResolvedAddressRequest> {
    let caller = ic_cdk::api::msg_caller();
    let index = req.index.unwrap_or(0);
    let account_tag = normalize_account_tag(req.account_tag)?;

    let mut derivation_path = vec![
        b"rustwalletforagent".to_vec(),
        b"network".to_vec(),
        network.as_bytes().to_vec(),
        b"caller".to_vec(),
        caller.as_slice().to_vec(),
        b"index".to_vec(),
        index.to_be_bytes().to_vec(),
    ];

    if let Some(tag) = &account_tag {
        derivation_path.push(b"account_tag".to_vec());
        derivation_path.push(tag.as_bytes().to_vec());
    }

    Ok(ResolvedAddressRequest {
        index,
        account_tag,
        derivation_path,
    })
}

pub async fn fetch_ecdsa_secp256k1_public_key(
    derivation_path: Vec<Vec<u8>>,
) -> WalletResult<(Vec<u8>, String)> {
    let key_name = config::app_config::default_ecdsa_key_name().to_string();
    let args = EcdsaPublicKeyArgs {
        canister_id: None,
        derivation_path,
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name.clone(),
        },
    };

    let result = management_canister::ecdsa_public_key(&args)
        .await
        .map_err(|err| WalletError::Internal(format!("ecdsa_public_key failed: {err}")))?;

    Ok((result.public_key, key_name))
}

pub async fn fetch_schnorr_public_key(
    algorithm: SchnorrAlgorithm,
    derivation_path: Vec<Vec<u8>>,
) -> WalletResult<(Vec<u8>, String)> {
    let key_name = config::app_config::default_schnorr_key_name().to_string();
    let args = SchnorrPublicKeyArgs {
        canister_id: None,
        derivation_path,
        key_id: SchnorrKeyId {
            algorithm,
            name: key_name.clone(),
        },
    };

    let result = management_canister::schnorr_public_key(&args)
        .await
        .map_err(|err| WalletError::Internal(format!("schnorr_public_key failed: {err}")))?;

    Ok((result.public_key, key_name))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn base58_encode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let mut zeros = 0usize;
    while zeros < bytes.len() && bytes[zeros] == 0 {
        zeros += 1;
    }

    let mut input = bytes.to_vec();
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    let mut start = zeros;

    while start < input.len() {
        let mut remainder: u32 = 0;
        for byte in input.iter_mut().skip(start) {
            let value = (remainder << 8) | (*byte as u32);
            *byte = (value / 58) as u8;
            remainder = value % 58;
        }
        encoded.push(BASE58_ALPHABET[remainder as usize]);
        while start < input.len() && input[start] == 0 {
            start += 1;
        }
    }

    for _ in 0..zeros {
        encoded.push(BASE58_ALPHABET[0]);
    }
    encoded.reverse();
    String::from_utf8(encoded).expect("base58 alphabet is valid utf-8")
}

pub fn encode_segwit_v1_bech32m(hrp: &str, witness_program: &[u8]) -> WalletResult<String> {
    if witness_program.len() != 32 {
        return Err(WalletError::invalid_input(
            "taproot witness program must be 32 bytes",
        ));
    }

    let mut data = vec![1u8];
    data.extend(convert_bits(witness_program, 8, 5, true)?);
    bech32m_encode(hrp, &data)
}

fn normalize_account_tag(account_tag: Option<String>) -> WalletResult<Option<String>> {
    let Some(tag) = account_tag else {
        return Ok(None);
    };
    let normalized = tag.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.len() > 64 {
        return Err(WalletError::invalid_input(
            "account_tag is too long (max 64 chars)",
        ));
    }
    Ok(Some(normalized))
}

fn convert_bits(data: &[u8], from_bits: u32, to_bits: u32, pad: bool) -> WalletResult<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let maxv: u32 = (1 << to_bits) - 1;
    let max_acc: u32 = (1 << (from_bits + to_bits - 1)) - 1;
    let mut out = Vec::with_capacity((data.len() * from_bits as usize).div_ceil(to_bits as usize));

    for value in data {
        let v = *value as u32;
        if v >> from_bits != 0 {
            return Err(WalletError::invalid_input("invalid bit group value"));
        }
        acc = ((acc << from_bits) | v) & max_acc;
        bits += from_bits;
        while bits >= to_bits {
            bits -= to_bits;
            out.push(((acc >> bits) & maxv) as u8);
        }
    }

    if pad {
        if bits > 0 {
            out.push(((acc << (to_bits - bits)) & maxv) as u8);
        }
    } else if bits >= from_bits || ((acc << (to_bits - bits)) & maxv) != 0 {
        return Err(WalletError::invalid_input("invalid padding"));
    }

    Ok(out)
}

fn bech32m_encode(hrp: &str, data: &[u8]) -> WalletResult<String> {
    if hrp.is_empty() {
        return Err(WalletError::invalid_input("bech32 hrp is required"));
    }
    if !hrp
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(WalletError::invalid_input(
            "bech32 hrp must be lowercase ascii",
        ));
    }
    if data.iter().any(|v| *v >= 32) {
        return Err(WalletError::invalid_input(
            "bech32 data values must be < 32",
        ));
    }

    let checksum = bech32m_checksum(hrp, data);
    let mut out = String::with_capacity(hrp.len() + 1 + data.len() + checksum.len());
    out.push_str(hrp);
    out.push('1');
    for &v in data {
        out.push(BECH32_CHARSET[v as usize] as char);
    }
    for &v in &checksum {
        out.push(BECH32_CHARSET[v as usize] as char);
    }
    Ok(out)
}

fn bech32m_checksum(hrp: &str, data: &[u8]) -> [u8; 6] {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    values.extend_from_slice(&[0u8; 6]);
    let polymod = bech32_polymod(&values) ^ BECH32M_CONST;
    let mut out = [0u8; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (5 - i))) & 0x1f) as u8;
    }
    out
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

    let mut chk: u32 = 1;
    for value in values {
        let b = (chk >> 25) as u8;
        chk = ((chk & 0x01ff_ffff) << 5) ^ (*value as u32);
        for (i, g) in GEN.iter().enumerate() {
            if ((b >> i) & 1) != 0 {
                chk ^= *g;
            }
        }
    }
    chk
}
