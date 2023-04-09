use cosmwasm_schema::{cw_serde, QueryResponses};
use outpost_utils::comp_prefs::{CompoundPrefs, JunoDestinationProject};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(VersionResponse)]
    Version {},
}

#[cw_serde]
pub struct VersionResponse {
    pub version: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Compound {
        /// list of pools to compound and how to compound each one
        pools: Vec<PoolCompoundPrefs>,
        /// comp prefs for any pool that was not specified in the pools list
        other_pools: Option<Vec<PoolCatchAllDestinationAction>>,
        /// Address of pools that the delegator is currently in.
        /// If this is not provided, the contract will query wyndex directly
        /// resulting in much higher gas usage.
        /// https://api.wynddao.com/pools/user/{delegator_address} can furnish this information off chain
        current_user_pools: Option<Vec<String>>,
        delegator_address: String,
    },
}

#[cw_serde]
/// compound prefs for a specific pool
pub struct PoolCompoundPrefs {
    pub pool_address: String,
    pub comp_prefs: CompoundPrefs,
}

#[cw_serde]
/// compound prefs for all of the pools that have rewards and were not
/// individually specified
pub struct PoolCatchAllDestinationAction {
    pub destination: PoolCatchAllDestinationProject,
    /// the percentage of the rewards that should be sent to this destination
    /// this is a number with 18 decimal places
    /// for example "250000000000000000" is 25%
    pub amount: u128,
}

#[cw_serde]
/// Compound prefs for a catch all pools that were not individually specified.
/// The main difference between this and the normal DestinationProject is that
/// in the catch all you have the ability to specify sending the rewards back to the pool
/// it came from instead of needing to specify any static destination
pub enum PoolCatchAllDestinationProject {
    BasicDestination(JunoDestinationProject),
    /// send pool rewards back to the pool that generated the rewards
    ReturnToPool,
}
