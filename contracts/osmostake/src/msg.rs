use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Decimal, Timestamp};
use cw_grant_spec::grants::{GrantRequirement, RevokeRequirement};

use osmosis_destinations::comp_prefs::{
    OsmosisCompPrefs, OsmosisDestinationProjectAddresses, OsmosisDestinationProjectAddrs,
};


use crate::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// Set the admin of the contract
    /// If none given it will be the contract creator
    pub admin: Option<String>,

    /// All of the addresses that the compounder can interact with
    pub project_addresses: ContractAddresses,
}

#[cw_serde]
pub struct MigrateMsg {
    pub project_addresses: Option<ContractAddresses>,
}

#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(VersionResponse)]
    Version {},

    #[returns(AuthorizedCompoundersResponse)]
    AuthorizedCompounders {},

    #[returns(Vec<GrantRequirement>)]
    GrantSpec {
        expiration: Timestamp,
        comp_prefs: OsmostakeCompoundPrefs,
    },

    #[returns(Vec<RevokeRequirement>)]
    RevokeSpec { comp_prefs: OsmostakeCompoundPrefs },
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
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    AddAuthorizedCompounder(String),
    RemoveAuthorizedCompounder(String),
    Compound(OsmostakeCompoundPrefs),
    UpdateProjectAddresses(Box<ContractAddresses>),
}

#[cw_serde]
pub struct OsmostakeCompoundPrefs {
    /// For now this should be an array of one item containing directives for compounding juno tokens only
    pub comp_prefs: OsmosisCompPrefs,
    pub user_address: String,
    pub tax_fee: Option<Decimal>,
}

#[cw_serde]
pub struct CompPrefsWithAddresses {
    pub comp_prefs: OsmostakeCompoundPrefs,
    pub project_addresses: ContractAddrs,
}

#[cw_serde]
pub struct ContractAddresses {
    pub staking_denom: String,
    pub take_rate_addr: String,
    pub authzpp: AuthzppAddresses,
    pub destination_projects: OsmosisDestinationProjectAddresses,
}

#[cw_serde]
#[derive(Default)]
pub struct AuthzppAddresses {
    pub withdraw_tax: String,
    // pub allowlist_send: String,
}

#[cw_serde]
pub struct AuthzppAddrs {
    pub withdraw_tax: Addr,
    // pub allowlist_send: Addr,
}
impl AuthzppAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<AuthzppAddrs, ContractError> {
        Ok(AuthzppAddrs {
            withdraw_tax: api.addr_validate(&self.withdraw_tax)?,
            // allowlist_send: api.addr_validate(&self.allowlist_send)?,
        })
    }
}

#[cw_serde]
pub struct ContractAddrs {
    pub staking_denom: String,
    pub take_rate_addr: Addr,
    pub authzpp: AuthzppAddrs,
    pub destination_projects: OsmosisDestinationProjectAddrs,
}

impl ContractAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<ContractAddrs, ContractError> {
        Ok(ContractAddrs {
            staking_denom: self.staking_denom.clone(),
            take_rate_addr: api.addr_validate(&self.take_rate_addr)?,
            authzpp: self.authzpp.validate_addrs(api)?,
            destination_projects: self.destination_projects.validate_addrs(api)?,
        })
    }
}
