use cosmos_sdk_proto::prost;
use cosmwasm_std::{CheckedMultiplyFractionError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Invalid prefs: Relative quantities must sum to 1")]
    InvalidPrefQtys,

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Could not query pendingRewards")]
    QueryPendingRewardsFailure,

    #[error("Could not generate exec message")]
    GenerateExecFailure,

    #[error("Could not encode msg as any")]
    EncodeError(#[from] prost::EncodeError),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
