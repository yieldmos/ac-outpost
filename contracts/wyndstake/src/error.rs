use cosmwasm_std::{CheckedMultiplyFractionError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    Std(#[from] StdError),

    #[error("Outpost Utils Error: &{0}")]
    OutpostError(#[from] outpost_utils::errors::OutpostError),

    #[error("Wynd Helper Error: &{0}")]
    WyndHelperError(#[from] wynd_helpers::errors::WyndHelperError),

    // #[error("Juno Helper Error: &{0}")]
    // JunoHelperError(#[from] juno_helpers::errors::HelperError),

    #[error("Juno Destinations Error: &{0}")]
    JunoDestinationError(#[from] juno_destinations::errors::JunoDestinationError),

    #[error("Sail Destinations Error: &{0}")]
    SailDestinationError(#[from] sail_destinations::errors::SailDestinationError),

    #[error("Universal Destinations Error: &{0}")]
    UniversalDestinationError(#[from] universal_destinations::errors::UniversalDestinationError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Failed to query Wynd Rewards. Err: {0}")]
    QueryWyndRewardsFailure(String),

    #[error("No DCA Compound Preferences found")]
    NoDCACompoundPrefs,

    #[error("Invalid DCA Compound Preferences: Only JUNO DCA currently allowed")]
    InvalidDCACompoundPrefs,

    #[error("Invalid Compound Preferences")]
    InvalidCompoundPrefs,

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("Target Not Implemented")]
    NotImplemented {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0} is not a valid address. Cannot set as authorized address")]
    InvalidAuthorizedAddress(String),

    #[error("{0} is already an authorized compounder")]
    DuplicateAuthorizedAddress(String),

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },

    #[error("Could not encode msg as any: {0}")]
    EncodeError(#[from] cosmos_sdk_proto::prost::EncodeError),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
