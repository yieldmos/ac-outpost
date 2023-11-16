use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use outpost_utils::osmosis_comp_prefs::OsmosisCompPrefs;

#[cw_serde]
pub struct OutpostAddresses {
    pub osmosis_swap_router_address: String,
    pub mars_red_bank_address: String,
    pub ion_dao_staking_address: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Set the admin of the contract
    /// If none given it will be the contract creator
    pub admin: Option<String>,
    pub outpost_addresses: OutpostAddresses,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(VersionResponse)]
    Version {},

    #[returns(AuthorizedCompoundersResponse)]
    AuthorizedCompounders {},
}

#[cw_serde]
pub struct AuthorizedCompoundersResponse {
    pub admin: Addr,
    pub authorized_compound_addresses: Vec<Addr>,
}

#[cw_serde]
pub struct VersionResponse {
    pub version: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddAuthorizedCompounder {
        address: String,
    },
    RemoveAuthorizedCompounder {
        address: String,
    },
    Compound {
        comp_prefs: OsmosisCompPrefs,
        delegator_address: String,
    },
}
