use cosmwasm_schema::{cw_serde, QueryResponses};
use outpost_utils::comp_prefs::{PoolCatchAllDestinationAction, PoolCompoundPrefs};
use wyndex::pair::PairInfo;

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
        current_user_pools: Option<Vec<PairInfo>>,
        delegator_address: String,
    },
}
