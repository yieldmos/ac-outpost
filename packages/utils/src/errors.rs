use cosmos_sdk_proto::prost;
use cosmwasm_std::{Decimal, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutpostError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Outpost admin could not be loaded")]
    AdminLoadFailure(),

    #[error("Outpost authorized admin could not be loaded")]
    AuthorizedAdminLoadFailure(),

    #[error("Invalid prefs: Relative quantities must sum to 1. {sum:?}")]
    InvalidPrefQtys { sum: Decimal },

    #[error("Prefs include a zero qty")]
    ZeroPrefs,

    #[error("Could not convert prefs to percentages")]
    PrefsToPercentagesFailure(u128),

    #[error("Could not generate exec message")]
    GenerateExecFailure,

    #[error("Could not encode msg as any: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Compound arithemtic overflow: {0}")]
    OverflowError(#[from] cosmwasm_std::OverflowError),

    #[error("Compounder not authorized: {0}")]
    UnauthorizedCompounder(String),

    #[error("Could not query pendingRewards")]
    QueryPendingRewardsFailure,

    #[error("Invalid asset: {denom} for project: {project}")]
    InvalidAsset { denom: String, project: String },
}
