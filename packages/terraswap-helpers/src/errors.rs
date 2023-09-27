use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerraswapHelperError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },
}
