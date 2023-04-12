use cosmos_sdk_proto::prost;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutpostError {
    #[error("Invalid prefs: Relative quantities must be non-zero and sum to 1")]
    InvalidPrefQtys,

    #[error("Could not generate exec message")]
    GenerateExecFailure,

    #[error("Could not encode msg as any: {0}")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Compound arithemtic overflow: {0}")]
    OverflowError(#[from] cosmwasm_std::OverflowError),

    #[error("Could not simulate swap of {from} to {to}")]
    SwapSimulationError { from: String, to: String },

    #[error("Parsing invalid wynd pool bonding period: {0}")]
    InvalidBondingPeriod(String),
}
