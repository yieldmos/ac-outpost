use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Uint128};
use outpost_utils::comp_prefs::CompoundPrefs;

use crate::errors::OsmosisDestinationError;

pub type OsmosisCompPrefs = CompoundPrefs<OsmosisDestinationProject>;

// #[cw_serde]
// pub enum RedBankLeverRewardTarget {
//     LeaveLiquid,
//     Reexpose,
//     Repay,
// }

#[cw_serde]
pub enum OsmosisDestinationProject {
    /// Stake the tokens to a given validator
    OsmosisStaking {
        validator_address: String,
    },

    /// Swap the given denom for the target denom and leave that token liquid.
    TokenSwap {
        target_denom: String,
    },

    /// Send tokens to a specific address
    SendTokens {
        denom: String,
        address: String,
    },

    /// Stake token to a dao
    DaoDaoStake {
        dao: OsmosisDao,
    },

    // /// Pay back borrowed balance. Currently the first denom strings specified in the vector will be
    // /// paid back first. No order is guaranteed when no vector is passed in.
    // /// Eventually there should be an option to pay back the highest cost debt first
    // RedBankPayback(PaybackDenoms),
    /// Deposit into redbank to potentially gain
    RedBankFundAccount {
        /*
                        {
                  "update_credit_account": {
                    "account_id": "13773",
                    "actions": [
                      {
                        "deposit": {
                          "denom": "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4",
                          "amount": "264953"
                        }
                      }
                    ]
                  }
                }

                {
          "update_credit_account": {
            "account_id": "13773",
            "actions": [
              {
                "deposit": {
                  "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                  "amount": "134694"
                }
              },
              {
                "lend": {
                  "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
                  "amount": "account_balance"
                }
              }
            ]
          }
        }
                */
        account_id: String,
        target_denom: String,
        lend_asset: bool,
    },
    /// Continuously lever up the given denom
    // RedBankLeverLoop {
    //     /// the denom to continuously lever up.
    //     /// at time of writing the options are atom, osmo, usdc, wbtc, weth
    //     denom: String,
    //     /// this is the percentage of the collateral that will be borrowed.
    //     /// should be a number with 18 places.
    //     /// defaults to 50%
    //     ltv_ratio: Option<u128>,
    // },

    /// Convert to Ion and stake it
    IonStaking {},

    // Swap to the appropriate pool tokens, join the pool, and lock the tokens for 14 days if desired
    OsmosisLiquidityPool {
        pool_id: u64,
        pool_settings: OsmosisPoolSettings,
    },

    MintLsd {
        lsd: OsmosisLsd,
    }, // RedBankVault {
    //     vault_address: String,
    //     leverage_amount: u64,
    // },
    /// Join the White Whale satellite market
    /// https://app.whitewhale.money/osmosis/dashboard
    // WhiteWhaleSatellite { asset: pool_network::asset::Asset },
    WhiteWhaleSatellite {
        asset: String,
    },
    Unallocated {},
}

#[cw_serde]
pub enum OsmosisPoolSettings {
    Standard {
        // If true, the pool tokens will be locked with a 14 day unbonding period
        bond_tokens: bool,
    },
    ConcentratedLiquidity {
        lower_tick: Uint128,
        upper_tick: Uint128,
        token_min_amount_0: Uint128,
        token_min_amount_1: Uint128,
    },
}

#[cw_serde]
pub enum OsmosisLsd {
    // https://www.erisprotocol.com/osmosis/amplifier/OSMO
    Eris,
    // https://app.milkyway.zone/
    MilkyWay,
}

// #[cw_serde]
// pub enum PaybackDenoms {
//     /// Pay back the given denoms only
//     Only(Vec<String>),
//     /// If no denom is set then pay back loans indiscriminately otherwise start with the given denom and then move onto the others
//     Any(Option<Vec<String>>),
// }

#[cw_serde]
#[derive(Default)]
pub struct OsmosisProjectAddresses {
    pub daodao: DaoDaoAddresses,
    pub redbank: RedbankAddresses,
    pub ion_dao: String,
    pub milky_way_bonding: String,
    pub eris_amposmo_bonding: String,
}
#[cw_serde]
pub struct OsmosisProjectAddrs {
    pub daodao: DaoDaoAddrs,
    pub redbank: RedbankAddrs,
    pub ion_dao: Addr,
    pub milky_way_bonding: Addr,
    pub eris_amposmo_bonding: Addr,
}
impl OsmosisProjectAddresses {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<OsmosisProjectAddrs, OsmosisDestinationError> {
        Ok(OsmosisProjectAddrs {
            daodao: self.daodao.validate_addrs(api)?,
            redbank: self.redbank.validate_addrs(api)?,
            ion_dao: api.addr_validate(&self.ion_dao)?,
            milky_way_bonding: api.addr_validate(&self.milky_way_bonding)?,
            eris_amposmo_bonding: api.addr_validate(&self.eris_amposmo_bonding)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct DestProjectSwapRoutes {
    pub osmo_tia_pool: u64,
    pub osmo_ion_pool: u64,
    pub osmo_mars_pool: u64,
    pub osmo_usdc_pool: u64,
    pub osmo_atom_pool: u64,
    pub osmo_whale_pool: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct OsmosisDestinationProjectAddresses {
    pub denoms: Denoms,
    pub swap_routes: DestProjectSwapRoutes,
    pub projects: OsmosisProjectAddresses,
}
#[cw_serde]
pub struct OsmosisDestinationProjectAddrs {
    pub denoms: Denoms,
    pub swap_routes: DestProjectSwapRoutes,
    pub projects: OsmosisProjectAddrs,
}
impl OsmosisDestinationProjectAddresses {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<OsmosisDestinationProjectAddrs, OsmosisDestinationError> {
        Ok(OsmosisDestinationProjectAddrs {
            denoms: self.denoms.clone(),
            swap_routes: self.swap_routes.clone(),
            projects: self.projects.validate_addrs(api)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct RedbankAddresses {
    deposit: String,
}
#[cw_serde]
pub struct RedbankAddrs {
    deposit: Addr,
}
impl RedbankAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<RedbankAddrs, OsmosisDestinationError> {
        Ok(RedbankAddrs {
            deposit: api.addr_validate(&self.deposit)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct Denoms {
    pub usdc: String,
    pub osmo: String,
    pub ion: String,
    pub tia: String,
    pub amp_osmo: String,
    pub mars: String,
    pub whale: String,
    pub amp_whale: String,
}

#[cw_serde]
pub enum OsmosisDao {}

#[cw_serde]
#[derive(Default)]
pub struct DaoAddress {
    pub denom: String,
    pub staking_address: String,
}
#[cw_serde]
pub struct DaoAddr {
    pub denom: String,
    pub staking_address: Addr,
}
impl DaoAddress {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<DaoAddr, OsmosisDestinationError> {
        Ok(DaoAddr {
            denom: self.denom.clone(),
            staking_address: api.addr_validate(&self.staking_address)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct DaoDaoAddresses {}
#[cw_serde]
pub struct DaoDaoAddrs {}
impl DaoDaoAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<DaoDaoAddrs, OsmosisDestinationError> {
        Ok(DaoDaoAddrs {})
    }
}
