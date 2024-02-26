use cosmwasm_std::{StdError, Timestamp, Uint64};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Unimplemented")]
    Unimplemented,

    #[error("Invalid admin address {0}")]
    InvalidAdminAddress(String),

    #[error("Invalid user address {0}")]
    InvalidUserAddress(String),

    #[error("Must set strategy setting on one's own behalf")]
    IncorrectCompPrefAddress,

    #[error("Invalid strategy id {0}")]
    InvalidStratId(Uint64),

    #[error("Invalid outpost address {0}")]
    InvalidOutpostAddress(String),

    #[error("Expiration cannot be earlier than the current time {0}")]
    InvalidExpiration(Timestamp),

    #[error("No previous settings found. User: {0}, Strategy Id: {1}")]
    NoSettingsFound(String, Uint64),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
