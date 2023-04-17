use cosmwasm_std::{CheckedMultiplyFractionError, DecimalRangeExceeded, OverflowError, StdError};
use thiserror::Error;
use wynd_helpers::errors::WyndHelperError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Outpost Utils: &{0}")]
    OutpostError(#[from] outpost_utils::errors::OutpostError),

    #[error("Wynd Helper Error: &{0}")]
    WyndHelperError(#[from] WyndHelperError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Target Not Implemented")]
    NotImplemented {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Could not query pendingRewards")]
    QueryPendingRewardsFailure,

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },

    #[error("Could not encode msg as any: {0}")]
    EncodeError(#[from] cosmos_sdk_proto::prost::EncodeError),

    #[error("Pool {pool} specified multiple times in compounding prefs")]
    DuplicatePoolPrefs { pool: String },

    #[error("Decimal out of range: {0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("No pool unbonding period found: {user} {pool}")]
    NoPoolUnbondingPeriod { user: String, pool: String },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
