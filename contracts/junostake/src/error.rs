use cosmos_sdk_proto::prost;
use cosmwasm_std::{CheckedMultiplyFractionError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Invalid prefs: Relative quantities must sum to 1")]
    InvalidPrefQtys,

    #[error("Target Not Implemented")]
    NotImplemented {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Could not query pendingRewards")]
    QueryPendingRewardsFailure,

    #[error("Could not generate exec message")]
    GenerateExecFailure,

    #[error("Could not encode msg as any")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
