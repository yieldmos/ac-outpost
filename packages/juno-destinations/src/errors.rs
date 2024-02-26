use cosmwasm_std::StdError;
use sail_destinations::errors::SailDestinationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JunoDestinationError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Juno Dest - Sail Destinations Error: &{0}")]
    SailDestinationsError(#[from] SailDestinationError),

    #[error("WyndHelper Error: {0}")]
    WyndHelperError(#[from] wynd_helpers::errors::WyndHelperError),

    #[error("Parsing invalid wynd pool bonding period: {0}")]
    InvalidBondingPeriod(String),

    #[error("Invalid asset: {denom} for project: {project}")]
    InvalidAsset { denom: String, project: String },
}
