use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use outpost_utils::juno_comp_prefs::JunoCompPrefs;

#[cw_serde]
pub struct InstantiateMsg {
    /// Set the admin of the contract
    /// If none given it will be the contract creator
    pub admin: Option<String>,
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
    AddAuthorizedCompounder(String),
    RemoveAuthorizedCompounder(String),
    Compound {
        comp_prefs: JunoCompPrefs,
        delegator_address: String,
    },
}
