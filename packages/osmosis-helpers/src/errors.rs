use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OsmosisHelperError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Pool not found: {pool_id}")]
    PoolNotFound { pool_id: u64 },

    #[error("Cannot enter pool {pool_id} because it has {pool_assets_len} assets")]
    PoolHasIncorrectAssetsNum { pool_id: u64, pool_assets_len: u64 },

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },
}
