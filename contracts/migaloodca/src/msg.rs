use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Coin, Decimal, Timestamp};
use cw_grant_spec::grants::{GrantRequirement, RevokeRequirement};
use migaloo_destinations::comp_prefs::{
    MigalooCompPrefs, MigalooDestinationProjectAddresses, MigalooDestinationProjectAddrs,
};
use outpost_utils::helpers::CompoundingFrequency;
use white_whale::pool_network::asset::AssetInfo;

use crate::ContractError;

#[cw_serde]
pub struct DcaPrefs {
    pub compound_token: Coin,
    pub compound_preferences: MigalooCompPrefs,
}

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
        frequency: CompoundingFrequency,
        expiration: Timestamp,
        comp_prefs: MigaloodcaCompoundPrefs,
    },

    #[returns(Vec<RevokeRequirement>)]
    RevokeSpec { comp_prefs: MigaloodcaCompoundPrefs },
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
    Compound(MigaloodcaCompoundPrefs),
    UpdateProjectAddresses(ContractAddresses),
}

#[cw_serde]
pub struct MigaloodcaCompoundPrefs {
    pub comp_prefs: Vec<DcaPrefs>,
    pub user_address: String,
    pub tax_fee: Option<Decimal>,
}

#[cw_serde]
pub struct CompPrefsWithAddresses {
    pub comp_prefs: MigaloodcaCompoundPrefs,
    pub project_addresses: ContractAddrs,
    pub comp_frequency: CompoundingFrequency,
}

#[cw_serde]
pub struct ContractAddresses {
    pub staking_denom: String,
    pub take_rate_addr: String,
    pub usdc: AssetInfo,
    pub authzpp: AuthzppAddresses,
    pub destination_projects: MigalooDestinationProjectAddresses,
}

#[cw_serde]
pub struct ContractAddrs {
    pub staking_denom: String,
    pub take_rate_addr: Addr,
    pub usdc: AssetInfo,
    pub authzpp: AuthzppAddrs,
    pub destination_projects: MigalooDestinationProjectAddrs,
}

impl ContractAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<ContractAddrs, ContractError> {
        Ok(ContractAddrs {
            staking_denom: self.staking_denom.clone(),
            take_rate_addr: api.addr_validate(&self.take_rate_addr)?,
            usdc: self.usdc.clone(),
            authzpp: self.authzpp.validate_addrs(api)?,
            destination_projects: self.destination_projects.validate_addrs(api)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct AuthzppAddresses {}

#[cw_serde]
pub struct AuthzppAddrs {}
impl AuthzppAddresses {
    pub fn validate_addrs(&self, _api: &dyn Api) -> Result<AuthzppAddrs, ContractError> {
        Ok(AuthzppAddrs {})
    }
}
