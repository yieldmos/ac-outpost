use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DestinationError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[cfg(feature = "juno")]
    #[error("WyndHelper Error: {0}")]
    WyndHelperError(#[from] wynd_helpers::errors::WyndHelperError),

    #[cfg(feature = "juno")]
    #[error("Parsing invalid wynd pool bonding period: {0}")]
    InvalidBondingPeriod(String),

    #[error("Invalid asset: {denom} for project: {project}")]
    InvalidAsset { denom: String, project: String },
}
