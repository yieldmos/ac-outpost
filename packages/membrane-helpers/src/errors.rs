use cosmwasm_std::StdError;
use osmosis_destinations::errors::OsmosisDestinationError;
use osmosis_helpers::errors::OsmosisHelperError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MembraneHelperError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Osmosis Destinations Error: &{0}")]
    OsmosisDestinationError(#[from] OsmosisDestinationError),

    #[error("Osmosis Helpers Error: &{0}")]
    OsmosisHelperError(#[from] OsmosisHelperError),
    // #[error("Divide by zero error: {0}")]
    // DivideByZeroError(#[from] DivideByZeroError),

    // #[error("Failed to convert int: {0}")]
    // TryFromIntError(#[from] std::num::TryFromIntError),
}
