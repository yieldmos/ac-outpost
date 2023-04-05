use cosmwasm_schema::{cw_serde, QueryResponses};
use outpost_utils::comp_prefs::CompoundPrefs;

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
        comp_prefs: CompoundPrefs,
        delegator_address: String,
    },
}
