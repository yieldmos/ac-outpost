use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JunoHelperError {
    #[error("Outpost StdError: {0}")]
    Std(#[from] StdError),

    #[error("WyndHelper Error: {0}")]
    WyndHelperError(#[from] wynd_helpers::errors::WyndHelperError),
}
