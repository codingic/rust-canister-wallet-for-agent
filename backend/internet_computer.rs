use candid::{CandidType, Nat, Principal};
use ic_cdk::call::Call;
use num_bigint::BigUint;
use serde::Deserialize;

use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredTokenResponse,
    TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::INTERNET_COMPUTER;
const ICP_DECIMALS: u8 = 8;

pub async fn request_address() -> WalletResult<AddressResponse> {
    let principal = current_canister_principal();
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: principal.to_text(),
        public_key_hex: String::new(),
        key_name: "canister_principal".to_string(),
        message: Some(
            "ICP/ICRC uses the backend canister principal as the default managed owner address"
                .to_string(),
        ),
    })
}

#[derive(CandidType, Deserialize, Clone, Debug)]
struct IcrcAccount {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
struct IcrcTransferArg {
    from_subaccount: Option<Vec<u8>>,
    to: IcrcAccount,
    fee: Option<Nat>,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
    amount: Nat,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
enum IcrcTransferError {
    BadFee { expected_fee: Nat },
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub async fn get_balance_icp(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account_text(&req.account)?;
    if non_empty_opt_str(req.token.as_deref()).is_some() {
        return Err(WalletError::invalid_input(
            "icp_get_balance_icp does not accept token parameter",
        ));
    }
    let ledger = icp_ledger_principal();
    let account = parse_icrc_account(&req.account)?;
    let decimals = fetch_icrc_decimals(ledger).await.unwrap_or(ICP_DECIMALS);
    let amount = icrc1_balance_of(ledger, account).await?;

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: None,
        amount: Some(format_nat_units(&amount, decimals)),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("icrc1_balance_of on ICP ledger".to_string()),
    })
}

pub async fn get_balance_icrc(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account_text(&req.account)?;
    let token_text = non_empty_opt_str(req.token.as_deref())
        .ok_or_else(|| WalletError::invalid_input("token ledger canister id is required"))?;
    let ledger = parse_principal_text(token_text, "token ledger canister id")?;
    let account = parse_icrc_account(&req.account)?;
    let decimals = fetch_icrc_decimals(ledger).await?;
    let amount = icrc1_balance_of(ledger, account).await?;

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: Some(token_text.to_string()),
        amount: Some(format_nat_units(&amount, decimals)),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("icrc1_balance_of on token ledger".to_string()),
    })
}

pub async fn transfer_icp(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer_basics(&req)?;
    if non_empty_opt_str(req.token.as_deref()).is_some() {
        return Err(WalletError::invalid_input(
            "icp_transfer_icp does not accept token parameter",
        ));
    }

    let ledger = icp_ledger_principal();
    let to = parse_icrc_account(&req.to)?;
    let decimals = fetch_icrc_decimals(ledger).await.unwrap_or(ICP_DECIMALS);
    let amount = parse_decimal_nat_units(req.amount.trim(), decimals)?;
    let from_owner = current_canister_principal();
    validate_from_if_present(req.from.as_deref(), from_owner)?;

    let block_index = icrc1_transfer(
        ledger,
        IcrcTransferArg {
            from_subaccount: None,
            to,
            fee: None,
            memo: parse_memo(req.memo.as_deref()),
            created_at_time: None,
            amount,
        },
    )
    .await?;

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: Some(block_index.to_string()),
        signed_tx: None,
        signed_tx_encoding: None,
        broadcast_request: None,
        message: "icrc1_transfer on ICP ledger accepted".to_string(),
    })
}

pub async fn transfer_icrc(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer_basics(&req)?;
    let token_text = non_empty_opt_str(req.token.as_deref())
        .ok_or_else(|| WalletError::invalid_input("token ledger canister id is required"))?;
    let ledger = parse_principal_text(token_text, "token ledger canister id")?;
    let to = parse_icrc_account(&req.to)?;
    let decimals = fetch_icrc_decimals(ledger).await?;
    let amount = parse_decimal_nat_units(req.amount.trim(), decimals)?;
    let from_owner = current_canister_principal();
    validate_from_if_present(req.from.as_deref(), from_owner)?;

    let block_index = icrc1_transfer(
        ledger,
        IcrcTransferArg {
            from_subaccount: None,
            to,
            fee: None,
            memo: parse_memo(req.memo.as_deref()),
            created_at_time: None,
            amount,
        },
    )
    .await?;

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: Some(block_index.to_string()),
        signed_tx: None,
        signed_tx_encoding: None,
        broadcast_request: None,
        message: "icrc1_transfer on token ledger accepted".to_string(),
    })
}

pub async fn discover_icrc_token(ledger_text: &str) -> WalletResult<ConfiguredTokenResponse> {
    let ledger = parse_principal_text(ledger_text, "token ledger canister id")?;
    let decimals = fetch_icrc_decimals(ledger).await?;
    let symbol = fetch_icrc_symbol(ledger).await?;
    let name = fetch_icrc_name(ledger).await?;
    Ok(ConfiguredTokenResponse {
        network: NETWORK_NAME.to_string(),
        symbol,
        name,
        token_address: ledger.to_text(),
        decimals: u64::from(decimals),
    })
}

fn icp_ledger_principal() -> Principal {
    if config::app_config::default_icp_ledger_use_mainnet() {
        config::app_config::icp_ledger_mainnet_principal()
    } else {
        config::app_config::icp_ledger_local_principal()
    }
}

fn current_canister_principal() -> Principal {
    ic_cdk::api::canister_self()
}

fn validate_account_text(account: &str) -> WalletResult<()> {
    if account.trim().is_empty() {
        return Err(WalletError::invalid_input("account is required"));
    }
    Ok(())
}

fn validate_transfer_basics(req: &TransferRequest) -> WalletResult<()> {
    if req.to.trim().is_empty() {
        return Err(WalletError::invalid_input("to is required"));
    }
    if req.amount.trim().is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    Ok(())
}

fn validate_from_if_present(from: Option<&str>, expected_owner: Principal) -> WalletResult<()> {
    let Some(from_text) = non_empty_opt_str(from) else {
        return Ok(());
    };
    let from_principal = parse_principal_text(from_text, "from principal")?;
    if from_principal != expected_owner {
        return Err(WalletError::invalid_input(
            "from does not match canister-managed ICP/ICRC owner principal",
        ));
    }
    Ok(())
}

fn parse_icrc_account(text: &str) -> WalletResult<IcrcAccount> {
    // Current frontend uses principal/canister-id text. Subaccount text parsing can be added later.
    let owner = parse_principal_text(text.trim(), "account principal")?;
    Ok(IcrcAccount {
        owner,
        subaccount: None,
    })
}

fn parse_principal_text(text: &str, field_name: &str) -> WalletResult<Principal> {
    Principal::from_text(text.trim()).map_err(|err| {
        WalletError::invalid_input(format!("{field_name} must be principal text: {err}"))
    })
}

fn parse_memo(memo: Option<&str>) -> Option<Vec<u8>> {
    non_empty_opt_str(memo).map(|m| m.as_bytes().to_vec())
}

fn non_empty_opt_str<'a>(value: Option<&'a str>) -> Option<&'a str> {
    value.and_then(|v| {
        let t = v.trim();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

async fn icrc1_balance_of(ledger: Principal, account: IcrcAccount) -> WalletResult<Nat> {
    let res = Call::bounded_wait(ledger, "icrc1_balance_of")
        .with_arg(account)
        .await
        .map_err(|err| WalletError::Internal(format!("icrc1_balance_of failed: {err:?}")))?;
    let (value,): (Nat,) = res
        .candid_tuple()
        .map_err(|err| WalletError::Internal(format!("icrc1_balance_of decode failed: {err:?}")))?;
    Ok(value)
}

async fn fetch_icrc_decimals(ledger: Principal) -> WalletResult<u8> {
    let res = Call::bounded_wait(ledger, "icrc1_decimals")
        .await
        .map_err(|err| WalletError::Internal(format!("icrc1_decimals failed: {err:?}")))?;
    let (value,): (u8,) = res
        .candid_tuple()
        .map_err(|err| WalletError::Internal(format!("icrc1_decimals decode failed: {err:?}")))?;
    Ok(value)
}

async fn fetch_icrc_symbol(ledger: Principal) -> WalletResult<String> {
    let res = Call::bounded_wait(ledger, "icrc1_symbol")
        .await
        .map_err(|err| WalletError::Internal(format!("icrc1_symbol failed: {err:?}")))?;
    let (value,): (String,) = res
        .candid_tuple()
        .map_err(|err| WalletError::Internal(format!("icrc1_symbol decode failed: {err:?}")))?;
    Ok(value)
}

async fn fetch_icrc_name(ledger: Principal) -> WalletResult<String> {
    let res = Call::bounded_wait(ledger, "icrc1_name")
        .await
        .map_err(|err| WalletError::Internal(format!("icrc1_name failed: {err:?}")))?;
    let (value,): (String,) = res
        .candid_tuple()
        .map_err(|err| WalletError::Internal(format!("icrc1_name decode failed: {err:?}")))?;
    Ok(value)
}

async fn icrc1_transfer(ledger: Principal, arg: IcrcTransferArg) -> WalletResult<Nat> {
    let res = Call::unbounded_wait(ledger, "icrc1_transfer")
        .with_arg(arg)
        .await
        .map_err(|err| WalletError::Internal(format!("icrc1_transfer call failed: {err:?}")))?;
    let (result,): (Result<Nat, IcrcTransferError>,) = res
        .candid_tuple()
        .map_err(|err| WalletError::Internal(format!("icrc1_transfer decode failed: {err:?}")))?;
    result.map_err(|err| WalletError::Internal(format!("icrc1_transfer rejected: {err:?}")))
}

fn format_nat_units(value: &Nat, decimals: u8) -> String {
    let mut digits = normalize_numeric_separators(&value.to_string());
    if decimals == 0 {
        return digits;
    }
    let d = usize::from(decimals);
    if digits == "0" {
        return "0".to_string();
    }
    if digits.len() <= d {
        let mut frac = String::with_capacity(d);
        frac.push_str(&"0".repeat(d - digits.len()));
        frac.push_str(&digits);
        while frac.ends_with('0') {
            frac.pop();
        }
        return if frac.is_empty() {
            "0".to_string()
        } else {
            format!("0.{frac}")
        };
    }
    let split = digits.len() - d;
    let frac = digits.split_off(split);
    let mut frac_trimmed = frac;
    while frac_trimmed.ends_with('0') {
        frac_trimmed.pop();
    }
    if frac_trimmed.is_empty() {
        digits
    } else {
        format!("{digits}.{frac_trimmed}")
    }
}

fn parse_decimal_nat_units(value: &str, decimals: u8) -> WalletResult<Nat> {
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
        return Err(WalletError::invalid_input("amount format is invalid"));
    }
    let whole = normalize_numeric_separators(whole);
    if !whole.is_empty() && !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input("amount must be decimal"));
    }
    let mut whole_num = if whole.is_empty() {
        BigUint::from(0u8)
    } else {
        BigUint::parse_bytes(whole.as_bytes(), 10)
            .ok_or_else(|| WalletError::invalid_input("amount parse failed"))?
    };

    let scale = BigUint::from(10u8).pow(u32::from(decimals));
    whole_num *= &scale;

    if let Some(frac_part) = frac {
        let frac_part = normalize_numeric_separators(frac_part);
        if !frac_part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(WalletError::invalid_input("amount must be decimal"));
        }
        if frac_part.len() > usize::from(decimals) {
            return Err(WalletError::invalid_input("too many decimal places"));
        }
        let mut frac_text = frac_part;
        while frac_text.len() < usize::from(decimals) {
            frac_text.push('0');
        }
        if !frac_text.is_empty() {
            let frac_num = BigUint::parse_bytes(frac_text.as_bytes(), 10)
                .ok_or_else(|| WalletError::invalid_input("amount parse failed"))?;
            whole_num += frac_num;
        }
    }

    Ok(Nat::from(whole_num))
}

fn normalize_numeric_separators(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|c| *c != '_' && *c != ',')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_nat_without_underscores() {
        assert_eq!(
            format_nat_units(&Nat::from(123_456_789u64), 8),
            "1.23456789"
        );
        assert_eq!(format_nat_units(&Nat::from(100_000_000u64), 8), "1");
        assert_eq!(format_nat_units(&Nat::from(10_000u64), 8), "0.0001");
    }

    #[test]
    fn parses_decimal_amount_with_optional_separators() {
        assert_eq!(
            parse_decimal_nat_units("1.23456789", 8)
                .unwrap()
                .to_string()
                .replace('_', ""),
            "123456789"
        );
        assert_eq!(
            parse_decimal_nat_units("1_000.0001", 8)
                .unwrap()
                .to_string()
                .replace('_', ""),
            "100000010000"
        );
        assert_eq!(
            parse_decimal_nat_units("1,000.0001", 8)
                .unwrap()
                .to_string()
                .replace('_', ""),
            "100000010000"
        );
    }
}
