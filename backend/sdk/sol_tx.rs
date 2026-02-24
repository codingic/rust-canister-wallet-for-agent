use ed25519_dalek::VerifyingKey;
use num_bigint::BigUint;
use sha2::{Digest, Sha256};

use crate::error::{WalletError, WalletResult};

const SOLANA_SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];
const SPL_TOKEN_PROGRAM_ID_BASE58: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SPL_ASSOCIATED_TOKEN_PROGRAM_ID_BASE58: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const PDA_MARKER: &[u8] = b"ProgramDerivedAddress";
const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn decode_solana_pubkey(value: &str) -> WalletResult<[u8; 32]> {
    let bytes = base58_decode(value.trim())?;
    bytes.try_into().map_err(|_| {
        WalletError::invalid_input("solana pubkey/blockhash must decode to 32 bytes (base58)")
    })
}

pub fn encode_system_transfer_message(
    from_pubkey: &[u8; 32],
    to_pubkey: &[u8; 32],
    recent_blockhash: &[u8; 32],
    lamports: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);

    out.push(1);
    out.push(0);
    out.push(1);

    encode_shortvec_len(3, &mut out);
    out.extend_from_slice(from_pubkey);
    out.extend_from_slice(to_pubkey);
    out.extend_from_slice(&SOLANA_SYSTEM_PROGRAM_ID);

    out.extend_from_slice(recent_blockhash);

    encode_shortvec_len(1, &mut out);

    out.push(2);
    encode_shortvec_len(2, &mut out);
    out.push(0);
    out.push(1);

    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());
    encode_shortvec_len(data.len(), &mut out);
    out.extend_from_slice(&data);

    out
}

pub fn encode_signed_transaction(signature: &[u8], message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + signature.len() + message.len());
    encode_shortvec_len(1, &mut out);
    out.extend_from_slice(signature);
    out.extend_from_slice(message);
    out
}

#[allow(clippy::too_many_arguments)]
pub fn encode_spl_transfer_checked_message(
    owner_pubkey: &[u8; 32],
    source_token_account: &[u8; 32],
    dest_token_account: &[u8; 32],
    destination_owner: &[u8; 32],
    mint: &[u8; 32],
    recent_blockhash: &[u8; 32],
    amount_raw: u64,
    decimals: u8,
    create_destination_ata: bool,
) -> WalletResult<Vec<u8>> {
    let token_program_id = decode_solana_pubkey(SPL_TOKEN_PROGRAM_ID_BASE58)?;
    let associated_token_program_id = if create_destination_ata {
        Some(decode_solana_pubkey(
            SPL_ASSOCIATED_TOKEN_PROGRAM_ID_BASE58,
        )?)
    } else {
        None
    };
    let mut out = Vec::with_capacity(320);

    out.push(1);
    out.push(0);
    out.push(if create_destination_ata { 5 } else { 2 });

    encode_shortvec_len(if create_destination_ata { 8 } else { 5 }, &mut out);
    out.extend_from_slice(owner_pubkey);
    out.extend_from_slice(source_token_account);
    out.extend_from_slice(dest_token_account);
    out.extend_from_slice(mint);
    if create_destination_ata {
        out.extend_from_slice(destination_owner);
        out.extend_from_slice(&SOLANA_SYSTEM_PROGRAM_ID);
    }
    out.extend_from_slice(&token_program_id);
    if let Some(ata_program_id) = associated_token_program_id {
        out.extend_from_slice(&ata_program_id);
    }

    out.extend_from_slice(recent_blockhash);

    encode_shortvec_len(if create_destination_ata { 2 } else { 1 }, &mut out);

    if create_destination_ata {
        out.push(7);
        encode_shortvec_len(6, &mut out);
        out.push(0);
        out.push(2);
        out.push(4);
        out.push(3);
        out.push(5);
        out.push(6);
        encode_shortvec_len(1, &mut out);
        out.push(1);
    }

    out.push(if create_destination_ata { 6 } else { 4 });
    encode_shortvec_len(4, &mut out);
    out.push(1);
    out.push(3);
    out.push(2);
    out.push(0);

    let mut data = Vec::with_capacity(10);
    data.push(12);
    data.extend_from_slice(&amount_raw.to_le_bytes());
    data.push(decimals);
    encode_shortvec_len(data.len(), &mut out);
    out.extend_from_slice(&data);

    Ok(out)
}

pub fn derive_associated_token_address(
    owner: &[u8; 32],
    mint: &[u8; 32],
) -> WalletResult<[u8; 32]> {
    let token_program_id = decode_solana_pubkey(SPL_TOKEN_PROGRAM_ID_BASE58)?;
    let ata_program_id = decode_solana_pubkey(SPL_ASSOCIATED_TOKEN_PROGRAM_ID_BASE58)?;
    find_program_address(
        &[
            owner.as_slice(),
            token_program_id.as_slice(),
            mint.as_slice(),
        ],
        &ata_program_id,
    )
}

fn base58_decode(input: &str) -> WalletResult<Vec<u8>> {
    if input.is_empty() {
        return Err(WalletError::invalid_input("base58 string is required"));
    }

    let mut zeros = 0usize;
    for ch in input.bytes() {
        if ch == BASE58_ALPHABET[0] {
            zeros += 1;
        } else {
            break;
        }
    }

    let mut acc = BigUint::from(0u8);
    for ch in input.bytes() {
        let digit = base58_digit(ch)
            .ok_or_else(|| WalletError::invalid_input("invalid base58 character"))?;
        acc = acc * 58u8 + BigUint::from(digit);
    }

    let mut decoded = acc.to_bytes_be();
    if zeros > 0 {
        let mut prefixed = vec![0u8; zeros];
        prefixed.append(&mut decoded);
        Ok(prefixed)
    } else {
        Ok(decoded)
    }
}

fn base58_digit(ch: u8) -> Option<u8> {
    BASE58_ALPHABET
        .iter()
        .position(|c| *c == ch)
        .map(|i| i as u8)
}

fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> WalletResult<[u8; 32]> {
    for bump in (0u8..=255u8).rev() {
        let bump_seed = [bump];
        let mut all_seeds = seeds.to_vec();
        all_seeds.push(&bump_seed);
        if let Some(addr) = try_create_program_address(&all_seeds, program_id)? {
            return Ok(addr);
        }
    }
    Err(WalletError::Internal(
        "failed to derive valid Solana program-derived address".into(),
    ))
}

fn try_create_program_address(
    seeds: &[&[u8]],
    program_id: &[u8; 32],
) -> WalletResult<Option<[u8; 32]>> {
    if seeds.iter().any(|s| s.len() > 32) {
        return Err(WalletError::invalid_input(
            "solana PDA seed length exceeds 32 bytes",
        ));
    }
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update(program_id);
    hasher.update(PDA_MARKER);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash[..32]);
    if is_on_ed25519_curve(&out) {
        return Ok(None);
    }
    Ok(Some(out))
}

fn is_on_ed25519_curve(candidate: &[u8; 32]) -> bool {
    VerifyingKey::from_bytes(candidate).is_ok()
}

fn encode_shortvec_len(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut elem = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(elem);
            break;
        }
        elem |= 0x80;
        out.push(elem);
    }
}
