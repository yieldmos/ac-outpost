use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use outpost_utils::juno_comp_prefs::JunoCompPrefs;

#[cw_serde]
pub struct InstantiateMsg {
    /// Set the admin of the contract
    /// If none given it will be the contract creator
    pub admin: Option<String>,

    /// All of the addresses that the compounder can interact with
    pub project_addresses: ProjectAddresses,
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
        tax_fee: Option<Decimal>,
    },
}

#[cw_serde]
pub struct ProjectAddresses {
    pub wynd_addresses: WyndAddresses,
    pub neta_addresses: NetaAddresses,
    pub gelotto_addresses: GelottoAddresses,
    pub authzpp_addresses: AuthzppAddresses,
}

#[cw_serde]
pub struct WyndAddresses {
    pub cw20: String,
    pub multihop: String,
    pub juno_neta_pair: String,
    pub juno_wynd_pair: String,
}

#[cw_serde]
pub struct NetaAddresses {
    pub cw20: String,
    pub staking: String,
}

#[cw_serde]
pub struct GelottoAddresses {}

#[cw_serde]
pub struct AuthzppAddresses {
    pub withdraw_tax: String,
    pub allowlist_send: String,
}
