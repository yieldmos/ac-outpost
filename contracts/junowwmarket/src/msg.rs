use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Decimal, Timestamp};
use cw_grant_spec::grants::{GrantRequirement, RevokeRequirement};
use outpost_utils::{
    helpers::CompoundingFrequency,

};
use white_whale::pool_network::{asset::AssetInfo as WWAssetInfo, router::SwapOperation};
use wyndex::asset::AssetInfo;
use juno_destinations::comp_prefs::{DestinationProjectAddresses, DestinationProjectAddrs, JunoCompPrefs};

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
        comp_prefs: JunoWhiteWhaleMarketCompoundPrefs,
        frequency: CompoundingFrequency,
    },

    #[returns(Vec<RevokeRequirement>)]
    RevokeSpec { comp_prefs: JunoWhiteWhaleMarketCompoundPrefs },
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
    Compound(JunoWhiteWhaleMarketCompoundPrefs),
    UpdateProjectAddresses(ContractAddresses),
}

#[cw_serde]
pub struct JunoWhiteWhaleMarketCompoundPrefs {
    pub comp_prefs: JunoCompPrefs,
    pub user_address: String,
    pub tax_fee: Option<Decimal>,
}

#[cw_serde]
pub struct CompPrefsWithAddresses {
    pub comp_prefs: JunoWhiteWhaleMarketCompoundPrefs,
    pub project_addresses: ContractAddrs,
}

#[cw_serde]
pub struct ContractAddresses {
    pub take_rate_addr: String,
    pub usdc: AssetInfo,
    pub authzpp: AuthzppAddresses,
    pub destination_projects: DestinationProjectAddresses,
    pub terraswap_routes: TerraswapRouteAddresses,
}

#[cw_serde]
pub struct ContractAddrs {
    pub take_rate_addr: Addr,
    pub usdc: AssetInfo,
    pub authzpp: AuthzppAddrs,
    pub destination_projects: DestinationProjectAddrs,

    pub terraswap_routes: TerraswapRouteAddrs,
}

impl ContractAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<ContractAddrs, ContractError> {
        Ok(ContractAddrs {
            take_rate_addr: api.addr_validate(&self.take_rate_addr)?,
            usdc: self.usdc.clone(),
            authzpp: self.authzpp.validate_addrs(api)?,
            destination_projects: self.destination_projects.validate_addrs(api)?,
            terraswap_routes: self.terraswap_routes.validate_addrs(api)?,
        })
    }
}

#[cw_serde]
pub struct TerraswapRouteAddresses {
    pub whale_usdc_pool: String,
    pub whale_ampwhale_pool: String,
    pub whale_bonewhale_pool: String,
    pub whale_rac_pool: String,

    pub whale_to_juno_route: Vec<SwapOperation>,
    pub whale_to_atom_route: Vec<SwapOperation>,

    pub usdc_asset: WWAssetInfo,
    pub ampwhale_asset: WWAssetInfo,
    pub bonewhale_asset: WWAssetInfo,
    pub juno_asset: WWAssetInfo,
    pub whale_asset: WWAssetInfo,
    pub rac_asset: WWAssetInfo,
    pub atom_asset: WWAssetInfo,
}

impl Default for TerraswapRouteAddresses {
    fn default() -> Self {
        TerraswapRouteAddresses {
            whale_usdc_pool: "".to_string(),
            whale_ampwhale_pool: "".to_string(),
            whale_bonewhale_pool: "".to_string(),
            whale_rac_pool: "".to_string(),
            whale_to_juno_route: vec![],
            whale_to_atom_route: vec![],

            usdc_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            ampwhale_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            bonewhale_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            juno_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            whale_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            rac_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
            atom_asset: WWAssetInfo::NativeToken { denom: "".to_string() },
        }
    }
}

#[cw_serde]
pub struct TerraswapRouteAddrs {
    pub whale_usdc_pool: Addr,
    pub whale_ampwhale_pool: Addr,
    pub whale_bonewhale_pool: Addr,
    pub whale_rac_pool: Addr,
    pub whale_to_juno_route: Vec<SwapOperation>,
    pub whale_to_atom_route: Vec<SwapOperation>,
    pub ampwhale_asset: WWAssetInfo,
    pub bonewhale_asset: WWAssetInfo,
    pub juno_asset: WWAssetInfo,
    pub usdc_asset: WWAssetInfo,
    pub whale_asset: WWAssetInfo,
    pub rac_asset: WWAssetInfo,
    pub atom_asset: WWAssetInfo,
}
impl TerraswapRouteAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<TerraswapRouteAddrs, ContractError> {
        Ok(TerraswapRouteAddrs {
            whale_usdc_pool: api.addr_validate(&self.whale_usdc_pool)?,
            whale_ampwhale_pool: api.addr_validate(&self.whale_ampwhale_pool)?,
            whale_bonewhale_pool: api.addr_validate(&self.whale_bonewhale_pool)?,
            whale_rac_pool: api.addr_validate(&self.whale_bonewhale_pool)?,
            whale_to_juno_route: self.whale_to_juno_route.clone(),
            whale_to_atom_route: self.whale_to_atom_route.clone(),
            usdc_asset: self.usdc_asset.clone(),
            ampwhale_asset: self.ampwhale_asset.clone(),
            bonewhale_asset: self.bonewhale_asset.clone(),
            juno_asset: self.juno_asset.clone(),
            whale_asset: self.whale_asset.clone(),
            rac_asset: self.rac_asset.clone(),
            atom_asset: self.atom_asset.clone(),
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct AuthzppAddresses {
    pub withdraw_tax: String,
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
