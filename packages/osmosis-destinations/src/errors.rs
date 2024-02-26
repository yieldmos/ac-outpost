use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OsmosisDestinationError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Invalid asset: {denom} for project: {project}")]
    InvalidAsset { denom: String, project: String },
}
