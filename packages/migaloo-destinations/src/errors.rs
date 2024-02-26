use cosmwasm_std::StdError;
use sail_destinations::errors::SailDestinationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MigalooDestinationError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Sail Destination Error in Migaloo Destinations: {0}")]
    SailDestinationError(#[from] SailDestinationError),

    #[error("LSD Mint Estimate Error: {error}. Project: {project}")]
    LsdMintEstimateError { error: String, project: String },

    #[error("Migaloo Destination Query Error: {error}. Project: {project}")]
    ProjectQueryError { error: String, project: String },

    #[error("Invalid asset: {denom} for project: {project}")]
    InvalidAsset { denom: String, project: String },
}
