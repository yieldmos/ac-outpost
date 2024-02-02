use cosmwasm_std::{DivideByZeroError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OsmosisHelperError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Divide by zero error: {0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("Pool not found: {pool_id}")]
    PoolNotFound { pool_id: u64 },

    #[error("Cannot enter pool {pool_id} because it has {pool_assets_len} assets")]
    PoolHasIncorrectAssetsNum { pool_id: u64, pool_assets_len: u64 },

    #[error("Invalid pool asset coins")]
    InvalidPoolAssetCoins,

    #[error("Cannot enter pool- incorrect assets")]
    InvalidAssets,

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },
}
