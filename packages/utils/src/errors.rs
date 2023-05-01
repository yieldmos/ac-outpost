use cosmos_sdk_proto::prost;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutpostError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Invalid prefs: Relative quantities must be non-zero and sum to 1")]
    InvalidPrefQtys,

    #[error("Could not generate exec message")]
    GenerateExecFailure,

    #[error("Could not encode msg as any: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Compound arithemtic overflow: {0}")]
    OverflowError(#[from] cosmwasm_std::OverflowError),

    #[error("Parsing invalid wynd pool bonding period: {0}")]
    InvalidBondingPeriod(String),

    #[error("Compounder not authorized: {0}")]
    UnauthorizedCompounder(String),

    #[error("Could not query pendingRewards")]
    QueryPendingRewardsFailure,
}
