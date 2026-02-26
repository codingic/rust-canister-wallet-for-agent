use num_bigint::BigUint;
use sha3::{Digest, Keccak256};

use crate::error::{WalletError, WalletResult};

pub fn encode_erc20_transfer_call(to: &[u8; 20], amount: &BigUint) -> WalletResult<Vec<u8>> {
    let amount_bytes = amount.to_bytes_be();
    if amount_bytes.len() > 32 {
        return Err(WalletError::invalid_input("token amount is too large"));
    }

    let mut out = Vec::with_capacity(4 + 32 + 32);
    out.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]); // transfer(address,uint256)
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(to);
    out.extend_from_slice(&vec![0u8; 32 - amount_bytes.len()]);
    out.extend_from_slice(&amount_bytes);
    Ok(out)
}

pub fn encode_erc20_balance_of_call(account: &[u8; 20]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 32);
    out.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]); // balanceOf(address)
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(account);
    out
}

pub fn parse_hex_quantity(hex: &str) -> WalletResult<BigUint> {
    let trimmed = hex.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .ok_or_else(|| WalletError::Internal("rpc result is not a hex quantity".to_string()))?;
    if digits.is_empty() {
        return Err(WalletError::Internal(
            "rpc result hex quantity is empty".to_string(),
        ));
    }
    BigUint::parse_bytes(digits.as_bytes(), 16)
        .ok_or_else(|| WalletError::Internal("rpc result hex quantity parse failed".to_string()))
}

pub fn parse_hex_data(hex: &str) -> WalletResult<Vec<u8>> {
    let trimmed = hex.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .ok_or_else(|| WalletError::Internal("rpc result is not hex data".to_string()))?;
    if digits.is_empty() {
        return Ok(Vec::new());
    }
    if digits.len() % 2 != 0 {
        return Err(WalletError::Internal(
            "rpc hex data length is odd".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let bytes = digits.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = from_hex_nibble(bytes[i])?;
        let lo = from_hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

pub fn parse_decimal_units(value: &str, decimals: usize) -> WalletResult<BigUint> {
    let v = value.trim();
    if v.is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    if v.starts_with('-') {
        return Err(WalletError::invalid_input("amount must be positive"));
    }
    let mut parts = v.split('.');
    let whole = parts.next().unwrap_or_default();
    let frac = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::invalid_input("invalid decimal amount format"));
    }
    if whole.is_empty() && frac.is_none() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    if !whole.is_empty() && !whole.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input(
            "amount has non-digit characters",
        ));
    }
    let frac = frac.unwrap_or("");
    if !frac.is_empty() && !frac.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input(
            "amount has non-digit characters",
        ));
    }
    if frac.len() > decimals {
        return Err(WalletError::invalid_input(format!(
            "amount supports at most {decimals} decimal places"
        )));
    }

    let whole_num = if whole.is_empty() {
        BigUint::from(0u8)
    } else {
        BigUint::parse_bytes(whole.as_bytes(), 10)
            .ok_or_else(|| WalletError::invalid_input("invalid whole amount"))?
    };
    let scale = BigUint::from(10u8).pow(decimals as u32);
    let mut out = &whole_num * &scale;

    if !frac.is_empty() {
        let mut frac_padded = frac.to_string();
        frac_padded.push_str(&"0".repeat(decimals - frac.len()));
        let frac_num = BigUint::parse_bytes(frac_padded.as_bytes(), 10)
            .ok_or_else(|| WalletError::invalid_input("invalid fractional amount"))?;
        out += frac_num;
    }
    Ok(out)
}

pub fn format_units(value: &BigUint, decimals: usize) -> String {
    if decimals == 0 {
        return value.to_str_radix(10);
    }

    let raw = value.to_str_radix(10);
    if raw == "0" {
        return "0".to_string();
    }

    if raw.len() <= decimals {
        let mut out = String::from("0.");
        out.push_str(&"0".repeat(decimals - raw.len()));
        out.push_str(&raw);
        trim_decimal_zeros(out)
    } else {
        let split = raw.len() - decimals;
        let mut out = String::with_capacity(raw.len() + 1);
        out.push_str(&raw[..split]);
        out.push('.');
        out.push_str(&raw[split..]);
        trim_decimal_zeros(out)
    }
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let digest = Keccak256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn rlp_encode_eip1559_unsigned(
    chain_id: u64,
    nonce: &BigUint,
    max_priority_fee_per_gas: &BigUint,
    max_fee_per_gas: &BigUint,
    gas_limit: &BigUint,
    to: &[u8; 20],
    value: &BigUint,
    data: &[u8],
) -> Vec<u8> {
    let payload = rlp_encode_list(&[
        rlp_encode_u64(chain_id),
        rlp_encode_biguint(nonce),
        rlp_encode_biguint(max_priority_fee_per_gas),
        rlp_encode_biguint(max_fee_per_gas),
        rlp_encode_biguint(gas_limit),
        rlp_encode_bytes(to),
        rlp_encode_biguint(value),
        rlp_encode_bytes(data),
        rlp_encode_list(&[]), // accessList
    ]);
    let mut out = Vec::with_capacity(1 + payload.len());
    out.push(0x02);
    out.extend_from_slice(&payload);
    out
}

pub fn rlp_encode_eip1559_signed(
    chain_id: u64,
    nonce: &BigUint,
    max_priority_fee_per_gas: &BigUint,
    max_fee_per_gas: &BigUint,
    gas_limit: &BigUint,
    to: &[u8; 20],
    value: &BigUint,
    data: &[u8],
    y_parity: u8,
    r: &BigUint,
    s: &BigUint,
) -> Vec<u8> {
    let payload = rlp_encode_list(&[
        rlp_encode_u64(chain_id),
        rlp_encode_biguint(nonce),
        rlp_encode_biguint(max_priority_fee_per_gas),
        rlp_encode_biguint(max_fee_per_gas),
        rlp_encode_biguint(gas_limit),
        rlp_encode_bytes(to),
        rlp_encode_biguint(value),
        rlp_encode_bytes(data),
        rlp_encode_list(&[]), // accessList
        rlp_encode_u64(u64::from(y_parity)),
        rlp_encode_biguint(r),
        rlp_encode_biguint(s),
    ]);
    let mut out = Vec::with_capacity(1 + payload.len());
    out.push(0x02);
    out.extend_from_slice(&payload);
    out
}

#[cfg(test)]
pub fn rlp_encode_u64_for_test(v: u64) -> Vec<u8> {
    rlp_encode_u64(v)
}

fn trim_decimal_zeros(mut s: String) -> String {
    if let Some(dot) = s.find('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.len() == dot + 1 {
            s.pop();
        }
    }
    s
}

fn from_hex_nibble(b: u8) -> WalletResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(WalletError::invalid_input("invalid hex character")),
    }
}

fn rlp_encode_u64(v: u64) -> Vec<u8> {
    rlp_encode_biguint(&BigUint::from(v))
}

fn rlp_encode_biguint(v: &BigUint) -> Vec<u8> {
    if *v == BigUint::from(0u8) {
        return rlp_encode_bytes(&[]);
    }
    rlp_encode_bytes(&v.to_bytes_be())
}

fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
    match data.len() {
        0 => vec![0x80],
        1 if data[0] < 0x80 => vec![data[0]],
        len if len <= 55 => {
            let mut out = Vec::with_capacity(1 + len);
            out.push(0x80 + len as u8);
            out.extend_from_slice(data);
            out
        }
        len => {
            let len_bytes = usize_to_be_bytes(len);
            let mut out = Vec::with_capacity(1 + len_bytes.len() + len);
            out.push(0xb7 + len_bytes.len() as u8);
            out.extend_from_slice(&len_bytes);
            out.extend_from_slice(data);
            out
        }
    }
}

fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload_len: usize = items.iter().map(Vec::len).sum();
    let mut payload = Vec::with_capacity(payload_len);
    for item in items {
        payload.extend_from_slice(item);
    }

    if payload_len <= 55 {
        let mut out = Vec::with_capacity(1 + payload_len);
        out.push(0xc0 + payload_len as u8);
        out.extend_from_slice(&payload);
        out
    } else {
        let len_bytes = usize_to_be_bytes(payload_len);
        let mut out = Vec::with_capacity(1 + len_bytes.len() + payload_len);
        out.push(0xf7 + len_bytes.len() as u8);
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(&payload);
        out
    }
}

fn usize_to_be_bytes(value: usize) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first_non_zero = bytes
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(bytes.len() - 1);
    bytes[first_non_zero..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_units_trims_trailing_zeros() {
        let v = BigUint::parse_bytes(b"1234500000000000000", 10).unwrap();
        assert_eq!(format_units(&v, 18), "1.2345");
    }

    #[test]
    fn parse_decimal_units_works() {
        let v = parse_decimal_units("0.001", 18).unwrap();
        assert_eq!(v.to_str_radix(10), "1000000000000000");
    }

    #[test]
    fn rlp_encodes_zero_as_empty_string() {
        assert_eq!(rlp_encode_u64_for_test(0), vec![0x80]);
    }

    #[test]
    fn eip1559_encoding_starts_with_type_prefix() {
        let zero = BigUint::from(0u8);
        let one = BigUint::from(1u8);
        let to = [0u8; 20];
        let unsigned = rlp_encode_eip1559_unsigned(1, &zero, &one, &one, &one, &to, &zero, &[]);
        let signed =
            rlp_encode_eip1559_signed(1, &zero, &one, &one, &one, &to, &zero, &[], 0, &one, &one);
        assert_eq!(unsigned.first().copied(), Some(0x02));
        assert_eq!(signed.first().copied(), Some(0x02));
    }
}
