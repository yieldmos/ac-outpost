use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Coin, Decimal, Timestamp, Uint64};
use cw_grant_spec::grants::{GrantRequirement, RevokeRequirement};

use osmosis_destinations::comp_prefs::{
    OsmosisCompPrefs, OsmosisDestinationProjectAddresses, OsmosisDestinationProjectAddrs,
};
use outpost_utils::{comp_prefs::TakeRate, helpers::CompoundingFrequency};

use crate::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    /// Set the admin of the contract
    /// If none given it will be the contract creator
    pub admin: Option<String>,

    /// All of the addresses that the compounder can interact with
    pub project_addresses: ContractAddresses,

    /// The maximum tax fee that can be charged to users of the contract
    pub max_tax_fee: Decimal,

    /// The address that the take rate should be sent to
    pub take_rate_address: String,

    /// The duration of the twap used for estimating the amount out for osmosis swaps
    pub twap_duration: Uint64,
}

#[cw_serde]
pub struct MigrateMsg {
    pub project_addresses: Option<ContractAddresses>,
    pub max_tax_fee: Decimal,
    pub take_rate_address: String,
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
        comp_prefs: OsmodcaCompoundPrefs,
    },

    #[returns(Vec<RevokeRequirement>)]
    RevokeSpec { comp_prefs: OsmodcaCompoundPrefs },

    #[returns(Uint64)]
    TwapDuration,
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
    Compound(OsmodcaCompoundPrefs),
    UpdateProjectAddresses(Box<ContractAddresses>),
    /// Update the number of seconds used for twap based osmosis swap estimates
    ChangeTwapDuration(Uint64),
}

#[cw_serde]
pub struct OsmodcaCompoundPrefs {
    /// For now this should be an array of one item containing directives for compounding juno tokens only
    pub comp_prefs: Vec<DcaPrefs>,
    pub user_address: String,
    pub tax_fee: Option<Decimal>,
}

#[cw_serde]
pub struct DcaPrefs {
    pub compound_token: Coin,
    pub compound_preferences: OsmosisCompPrefs,
}

#[cw_serde]
pub struct CompPrefsWithAddresses {
    pub comp_prefs: OsmodcaCompoundPrefs,
    pub project_addresses: ContractAddrs,
    pub comp_frequency: CompoundingFrequency,
    pub take_rate: TakeRate,
}

#[cw_serde]
pub struct ContractAddresses {
    pub authzpp: AuthzppAddresses,
    pub destination_projects: OsmosisDestinationProjectAddresses,
}

#[cw_serde]
#[derive(Default)]
pub struct AuthzppAddresses {
    // pub withdraw_tax: String,
    // pub allowlist_send: String,
}

#[cw_serde]
pub struct AuthzppAddrs {
    // pub withdraw_tax: Addr,
    // pub allowlist_send: Addr,
}
impl AuthzppAddresses {
    pub fn validate_addrs(&self, _api: &dyn Api) -> Result<AuthzppAddrs, ContractError> {
        Ok(AuthzppAddrs {
            // withdraw_tax: api.addr_validate(&self.withdraw_tax)?,
            // allowlist_send: api.addr_validate(&self.allowlist_send)?,
        })
    }
}

#[cw_serde]
pub struct ContractAddrs {
    pub authzpp: AuthzppAddrs,
    pub destination_projects: OsmosisDestinationProjectAddrs,
}

impl ContractAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<ContractAddrs, ContractError> {
        Ok(ContractAddrs {
            authzpp: self.authzpp.validate_addrs(api)?,
            destination_projects: self.destination_projects.validate_addrs(api)?,
        })
    }
}
