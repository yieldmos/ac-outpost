use cosmwasm_std::{CheckedMultiplyFractionError, Decimal, DecimalRangeExceeded, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Outpost Utils Error: &{0}")]
    OutpostError(#[from] outpost_utils::errors::OutpostError),

    #[error("Osmosis Helper Error: &{0}")]
    WyndHelperError(#[from] osmosis_helpers::errors::OsmosisHelperError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

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

    #[error("Red Bank deposits disabled for asset: {0}")]
    DepositDisabled(String),

    #[error("Red Bank target_ltv too high: {user_ltv} > {max_ltv}")]
    LTVTooHigh { user_ltv: Decimal, max_ltv: Decimal },

    #[error("Decimal Range Exceeded: {0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
