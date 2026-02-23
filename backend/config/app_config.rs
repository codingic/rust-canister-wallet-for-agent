use candid::Principal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    Dev,
    Prod,
}

pub const MODE: AppMode = AppMode::Dev;

pub const ICP_LEDGER_MAINNET_PRINCIPAL_TEXT: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const ICP_LEDGER_LOCAL_PRINCIPAL_TEXT: &str = "umunu-kh777-77774-qaaca-cai";

pub fn is_dev_mode() -> bool {
    matches!(MODE, AppMode::Dev)
}

pub fn auth_enabled() -> bool {
    !is_dev_mode()
}

pub fn icp_ledger_mainnet_principal() -> Principal {
    Principal::from_text(ICP_LEDGER_MAINNET_PRINCIPAL_TEXT)
        .expect("invalid ICP_LEDGER_MAINNET_PRINCIPAL_TEXT")
}

pub fn icp_ledger_local_principal() -> Principal {
    Principal::from_text(ICP_LEDGER_LOCAL_PRINCIPAL_TEXT)
        .expect("invalid ICP_LEDGER_LOCAL_PRINCIPAL_TEXT")
}

pub fn default_icp_ledger_use_mainnet() -> bool {
    matches!(MODE, AppMode::Prod)
}

pub fn default_http_cycles() -> u64 {
    match MODE {
        AppMode::Dev | AppMode::Prod => 30_000_000_000,
    }
}

pub fn default_ecdsa_key_name() -> &'static str {
    match MODE {
        AppMode::Dev => "dfx_test_key",
        AppMode::Prod => "key_1",
    }
}

pub fn default_schnorr_key_name() -> &'static str {
    match MODE {
        // dfx local replica commonly exposes `test_key_1` for threshold Schnorr APIs.
        AppMode::Dev => "test_key_1",
        AppMode::Prod => "key_1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_mode_defaults_match_motoko_config() {
        assert!(is_dev_mode());
        assert!(!auth_enabled());
        assert!(!default_icp_ledger_use_mainnet());
        assert_eq!(default_http_cycles(), 30_000_000_000);
        assert_eq!(default_ecdsa_key_name(), "dfx_test_key");
        assert_eq!(default_schnorr_key_name(), "test_key_1");
    }
}
