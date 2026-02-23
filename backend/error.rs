use candid::CandidType;
use serde::Deserialize;

pub type WalletResult<T> = Result<T, WalletError>;

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum WalletError {
    Forbidden,
    Paused,
    InvalidInput(String),
    Unimplemented { network: String, operation: String },
    Internal(String),
}

impl WalletError {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }
}
