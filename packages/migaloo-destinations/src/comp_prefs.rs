use crate::errors::MigalooDestinationError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, Uint128, Uint64};
use outpost_utils::comp_prefs::CompoundPrefs;
use sail_destinations::comp_prefs::{FundMsg, RacoonBetGame};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo},
    router::SwapOperation,
};

pub type MigalooCompPrefs = CompoundPrefs<MigalooDestinationProject>;

#[cw_serde]
pub enum MigalooDestinationProject {
    /// Native staking
    MigalooStaking {
        validator_address: String,
    },

    /// Send tokens to a specific address
    SendTokens {
        denom: AssetInfo,
        address: String,
    },

    // /// Swap and ecosystem stake
    // EcosystemStake {
    //     asset: EcosystemStakeAsset,
    // },
    /// Swapping to an abitrary token via TerraSwap
    TokenSwap {
        target_denom: AssetInfo,
    },

    /// Stake via Alliance Module
    AllianceStake {
        asset: AllianceAsset,
        validator_address: String,
    },

    DaoDaoStake {
        dao: MigalooDao,
    },

    /// Spark IBC Campaign Funding
    /// https://sparkibc.zone/earn
    SparkIbcCampaign {
        fund: FundMsg,
    },

    /// Join the White Whale satellite market
    /// https://app.whitewhale.money/juno/dashboard
    // WhiteWhaleSatellite { asset: pool_network::asset::Asset },
    WhiteWhaleSatellite {
        asset: AssetInfo,
    },

    /// Table Games on RacoonBet
    /// https://www.racoon.bet/
    RacoonBet {
        game: RacoonBetGame,
    },

    /// Mint one of many Migaloo/WHALE LSDs
    MintLsd {
        lsd_type: WhaleLsd,
        and_then: Option<LsdMintAction>,
    },

    /// Burn WHALE and receive ASH
    /// https://whale.burn.community/
    /// migaloo1erul6xyq0gk6ws98ncj7lnq9l4jn4gnnu9we73gdz78yyl2lr7qqrvcgup
    /// TODO: add guppy burn
    Furnace {
        and_then: Option<AshAction>,
    },

    /// Fund vaults on Migaloo
    Vault {
        vault: MigalooVault,
    },

    /// Ginkou usdc deposit
    GinkouDepositUSDC {
        and_then: Option<MUsdcAction>,
    },

    /// Ginkou provide liquidity
    /// https://ginkou.io/mypage/borrow
    GinkouProvideLiquidity {
        asset: AssetInfo,
        and_then: Option<GinkouBorrow>,
    },

    /// Ginku repay loan
    GinkouRepayLoan {},

    // /// TODO: Provide Liquidity to a TerraSwap pool
    // ProvideLiquidity {
    //     pool_address: String,
    //     pool_asset1: AssetInfo,
    //     pool_asset2: AssetInfo,
    //     unlock_duration: Uint128,
    //     // this is basically only for the whale usdc pool that can be used for ecosystem staking
    //     and_then: Option<ProvideLiquidityAction>,
    // },

    // TODO: whale usdc alliance
    /// Do nothing with the funds
    Unallocated {},
}

// native staking rewards, alliance rewards, lp rewards, sat market rewards

// #[cw_serde]
// pub enum EcosystemStakeAsset {
//     WhaleUsdcPool,
//     Ash,

// }

#[cw_serde]
pub enum MigalooDao {
    /// https://daodao.zone/dao/migaloo1nsh9t4uhnhzpgp79tx8mlcgx7ma9zl873685pemjhzckupf4vrssphzek6/home
    RacoonSupply,

    /// https://daodao.zone/dao/migaloo1mzxe5q5ry0kkajvf4mrytdvxfe66ep3jsx92fav6aef0xe2ckupqz97uce/home
    GuppyDao,
}

pub struct DaoDaoStakingInfo {
    pub dao_name: String,
    pub dao_addr: Addr,
    pub swap_pair_addr: Addr,
    pub asset_info: AssetInfo,
}

impl MigalooDao {
    pub fn staking_info(&self, addrs: &MigalooDestinationProjectAddrs) -> DaoDaoStakingInfo {
        match self {
            MigalooDao::GuppyDao => {
                unimplemented!();
                // DaoDaoStakingInfo {
                //     dao_name: "GUPPY DAO".to_string(),
                //     dao_addr: addrs.projects.daodao.guppy_dao.staking_address.clone(),
                //     swap_pair_addr: addrs.swap_routes.whale_guppy_pool.clone(),
                //     asset_info: AssetInfo::NativeToken {
                //         denom: addrs.denoms.guppy.clone(),
                //     },
                // }
            }
            MigalooDao::RacoonSupply => DaoDaoStakingInfo {
                dao_name: "$RAC DAO".to_string(),
                dao_addr: addrs
                    .projects
                    .daodao
                    .racoon_supply_dao
                    .staking_address
                    .clone(),
                swap_pair_addr: addrs.swap_routes.whale_rac_pool.clone(),
                asset_info: AssetInfo::NativeToken {
                    denom: addrs.denoms.rac.clone(),
                },
            },
        }
    }
}

#[cw_serde]
pub enum AllianceAsset {
    BLuna,
    AmpLuna,
}

#[cw_serde]
pub enum MUsdcAction {
    EcosystemStake,
    AmpUsdc,
}

#[cw_serde]
pub struct GinkouBorrow {
    desired_ltv: Decimal,
    action: GinkouBorrowAction,
}

#[cw_serde]
pub enum GinkouBorrowAction {
    GinkouDeposit { ecosystem_stake: bool },
    AmpUsdc,
    None,
}

#[cw_serde]
pub enum LsdMintAction {
    SatelliteMarket,
    // GinkouProvideLiquidity
}

#[cw_serde]
pub enum AshAction {
    EcosystemStake,
    AmpAsh,
    // ProvideLiquidity { pool_address: String },
}

#[cw_serde]
pub enum WhaleLsd {
    /// Mint Backbone's liquid staked Juno token (bWHALE)
    /// https://juno.gravedigger.zone/
    Backbone,

    /// Mint Whale LSD on Eris Protocol (ampWHALE)
    /// https://www.erisprotocol.com/migaloo/amplifier
    Eris,
}
impl WhaleLsd {
    pub fn get_mint_address(&self, addresses: &WhaleLsdAddrs) -> Addr {
        match self {
            WhaleLsd::Backbone => addresses.bone_whale.clone(),

            WhaleLsd::Eris => addresses.amp_whale.clone(),
        }
    }
    pub fn get_project_name(&self) -> String {
        match self {
            WhaleLsd::Backbone => "Backbone".to_string(),

            WhaleLsd::Eris => "Eris".to_string(),
        }
    }
    pub fn get_asset_info(&self, denoms: &Denoms) -> AssetInfo {
        match self {
            WhaleLsd::Backbone => AssetInfo::NativeToken {
                denom: denoms.bwhale.clone(),
            },

            WhaleLsd::Eris => AssetInfo::NativeToken {
                denom: denoms.ampwhale.clone(),
            },
        }
    }
    pub fn get_whale_pool_addr(&self, swap_routes: &DestProjectVerifiedSwapRoutes) -> Addr {
        match self {
            WhaleLsd::Backbone => swap_routes.whale_bwhale_pool.clone(),
            WhaleLsd::Eris => swap_routes.whale_ampwhale_pool.clone(),
        }
    }
}

impl std::fmt::Display for WhaleLsd {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            WhaleLsd::Backbone => write!(f, "boneWHALE"),
            WhaleLsd::Eris => write!(f, "ampWHALE"),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct WhaleLsdAddresses {
    ///
    pub bone_whale: String,

    ///
    pub amp_whale: String,
}
#[cw_serde]
pub struct WhaleLsdAddrs {
    pub bone_whale: Addr,
    pub amp_whale: Addr,
}
impl WhaleLsdAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<WhaleLsdAddrs, MigalooDestinationError> {
        Ok(WhaleLsdAddrs {
            bone_whale: api.addr_validate(&self.bone_whale)?,
            amp_whale: api.addr_validate(&self.amp_whale)?,
        })
    }
}

#[cw_serde]
pub enum MigalooVault {
    /// Eris Liquid Staked USDC
    AmpUsdc,

    /// Eris WHALE Arb Vault
    ArbWhale,

    /// Eris Liquid Staked ASH
    AmpAsh,
}

#[cw_serde]
#[derive(Default)]
pub struct Denoms {
    /// ibc/80E8F826480B995AE28C1EE86106C1BE2034FF1966579D29951CF61885458040
    pub usdc: String,

    /// uwhale
    pub whale: String,

    /// factory/migaloo1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqdhts4u/boneWhale
    pub bwhale: String,

    /// factory/migaloo1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgshqdky4/ampWHALE
    pub ampwhale: String,

    /// factory/migaloo1ey4sn2mkmhew4pdrzk90l9acluvas25qlhuvsfgssw42ugz8yjlqx92j9l/arbWHALE
    pub arbwhale: String,

    /// factory/migaloo1erul6xyq0gk6ws98ncj7lnq9l4jn4gnnu9we73gdz78yyl2lr7qqrvcgup/ash
    pub ash: String,

    /// factory/migaloo1etlu2h30tjvv8rfa4fwdc43c92f6ul5w9acxzk/uguppy
    pub guppy: String,

    /// factory/migaloo1eqntnl6tzcj9h86psg4y4h6hh05g2h9nj8e09l/urac
    pub rac: String,

    /// migaloo10nucfm2zqgzqmy7y7ls398t58pjt9cwjsvpy88y2nvamtl34rgmqt5em2v
    pub musdc: String,

    /// ibc/40C29143BF4153B365089E40E437B7AA819672646C45BB0A5F1E10915A0B6708
    pub bluna: String,

    /// ibc/05238E98A143496C8AF2B6067BABC84503909ECE9E45FBCBAC2CBA5C889FD82A
    pub ampluna: String,
}

#[cw_serde]
#[derive(Default)]
pub struct DestProjectSwapRoutes {
    /// migaloo1nha6qlam4p92cf9j64qv9he40xyf3akl2m9w5dukftmf2ryrxz5qy650zh
    pub whale_usdc_pool: String,

    /// migaloo1dg5jrt89nddtymjx5pzrvdvdt0m4zl3l2l3ytunl6a0kqd7k8hss594wy6
    pub whale_bwhale_pool: String,

    /// migaloo1ull9s4el2pmkdevdgrjt6pwa4e5xhkda40w84kghftnlxg4h3knqpm5u3n
    pub whale_ampwhale_pool: String,

    /// migaloo1ull9s4el2pmkdevdgrjt6pwa4e5xhkda40w84kghftnlxg4h3knqpm5u3n
    pub whale_ash_pool: String,

    // ///
    // pub whale_guppy_pool: String,
    /// migaloo1crsvm4qddplxhag29nd2zyw6k6jzh06hlcctya4ynfvuhhu3yt4q0pn4t3
    pub whale_rac_pool: String,

    /// From WHALE to something else
    pub whale: WhaleRoutes,
    pub usdc: UsdcRoutes,
}
impl DestProjectSwapRoutes {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<DestProjectVerifiedSwapRoutes, MigalooDestinationError> {
        Ok(DestProjectVerifiedSwapRoutes {
            whale_usdc_pool: api.addr_validate(&self.whale_usdc_pool)?,
            whale_bwhale_pool: api.addr_validate(&self.whale_bwhale_pool)?,
            whale_ampwhale_pool: api.addr_validate(&self.whale_ampwhale_pool)?,
            whale_ash_pool: api.addr_validate(&self.whale_ash_pool)?,
            // whale_guppy_pool: api.addr_validate(&self.whale_guppy_pool)?,
            whale_rac_pool: api.addr_validate(&self.whale_rac_pool)?,
            whale: self.whale.clone(),
            usdc: self.usdc.clone(),
        })
    }
}

#[cw_serde]
pub struct DestProjectVerifiedSwapRoutes {
    pub whale_usdc_pool: Addr,
    pub whale_bwhale_pool: Addr,
    pub whale_ampwhale_pool: Addr,
    pub whale_ash_pool: Addr,
    // pub whale_guppy_pool: Addr,
    pub whale_rac_pool: Addr,
    /// From WHALE to something else
    pub whale: WhaleRoutes,
    pub usdc: UsdcRoutes,
}

#[cw_serde]
#[derive(Default)]
pub struct WhaleRoutes {}

#[cw_serde]
#[derive(Default)]
pub struct UsdcRoutes {
    pub whale: Vec<SwapOperation>,
    // pub guppy: Vec<SwapOperation>,
}

#[cw_serde]
#[derive(Default)]
pub struct MigalooDestinationProjectAddresses {
    pub denoms: Denoms,
    pub swap_routes: DestProjectSwapRoutes,
    pub projects: MigalooProjectAddresses,
}
#[cw_serde]
pub struct MigalooDestinationProjectAddrs {
    pub denoms: Denoms,
    pub swap_routes: DestProjectVerifiedSwapRoutes,
    pub projects: MigalooProjectAddrs,
}
impl MigalooDestinationProjectAddresses {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<MigalooDestinationProjectAddrs, MigalooDestinationError> {
        Ok(MigalooDestinationProjectAddrs {
            denoms: self.denoms.clone(),
            swap_routes: self.swap_routes.validate_addrs(api)?,
            projects: self.projects.validate_addrs(api)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct MigalooProjectAddresses {
    /// migaloo1tma28exp38q92c69r8uujhphxy95xa4awq2cudqqg3nhzkhnrg5s4r60en
    pub terraswap_multihop_router: String,
    pub daodao: DaoDaoAddresses,
    /// migaloo1gxw00ht2jz490lkg46a884l7knz7sspk0djffsmlcnsf8a0cdnksnj2y0j
    pub spark_ibc: String,
    pub white_whale_satellite: SatelliteMarketAddresses,
    /// migaloo1vjt5hsgneptmdtxyadm440qdf6r5y56fmurf3k870qrvjk4pfgxqapwlsm
    pub racoon_bet: String,
    pub whale_lsd: WhaleLsdAddresses,
    /// migaloo1erul6xyq0gk6ws98ncj7lnq9l4jn4gnnu9we73gdz78yyl2lr7qqrvcgup
    pub furnace: String,
    pub vaults: VaultAddresses,
    pub ginkou: GinkouAddresses,
    /// migaloo190qz7q5fu4079svf890h4h3f8u46ty6cxnlt78eh486k9qm995hquuv9kd
    pub ecosystem_stake: String,
}
#[cw_serde]
pub struct MigalooProjectAddrs {
    pub terraswap_multihop_router: Addr,
    pub daodao: DaoDaoAddrs,
    pub spark_ibc: Addr,
    pub white_whale_satellite: SatelliteMarketAddrs,
    pub racoon_bet: Addr,
    pub whale_lsd: WhaleLsdAddrs,
    pub furnace: Addr,
    pub vaults: VaultAddrs,
    pub ginkou: GinkouAddrs,
    pub ecosystem_stake: Addr,
}
impl MigalooProjectAddresses {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<MigalooProjectAddrs, MigalooDestinationError> {
        Ok(MigalooProjectAddrs {
            terraswap_multihop_router: api.addr_validate(&self.terraswap_multihop_router)?,
            daodao: self.daodao.validate_addrs(api)?,
            spark_ibc: api.addr_validate(&self.spark_ibc)?,
            white_whale_satellite: self.white_whale_satellite.validate_addrs(api)?,
            racoon_bet: api.addr_validate(&self.racoon_bet)?,
            whale_lsd: self.whale_lsd.validate_addrs(api)?,
            furnace: api.addr_validate(&self.furnace)?,
            vaults: self.vaults.validate_addrs(api)?,
            ginkou: self.ginkou.validate_addrs(api)?,
            ecosystem_stake: api.addr_validate(&self.ecosystem_stake)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GinkouAddresses {
    /// migaloo1qelh4gv5drg3yhj282l6n84a6wrrz033kwyak3ee3syvqg3mu3msgphpk4
    pub deposit: String,
    /// migaloo10nucfm2zqgzqmy7y7ls398t58pjt9cwjsvpy88y2nvamtl34rgmqt5em2v
    pub borrow: String,
    // pub provide_liquidity: String,
}
#[cw_serde]
pub struct GinkouAddrs {
    pub deposit: Addr,
    pub borrow: Addr,
    // pub provide_liquidity: Addr,
}
impl GinkouAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<GinkouAddrs, MigalooDestinationError> {
        Ok(GinkouAddrs {
            deposit: api.addr_validate(&self.deposit)?,
            borrow: api.addr_validate(&self.borrow)?,
            // provide_liquidity: api.addr_validate(&self.provide_liquidity)?,
        })
    }
}

#[cw_serde]
#[derive(Default)]
pub struct VaultAddresses {
    /// migaloo12ye2j33d6lv84x8zq6dpjj2hepzn2njnrnnwlmuam0v0eczr787qhmf7en
    pub amp_usdc: String,

    /// migaloo1ey4sn2mkmhew4pdrzk90l9acluvas25qlhuvsfgssw42ugz8yjlqx92j9l
    pub arb_whale: String,

    /// migaloo1cmcnld5q4z9nltml664nuxthcrz5r9vpfv0efgadxj4pwl3ry8yq26nk76
    pub amp_ash: String,
}
#[cw_serde]
pub struct VaultAddrs {
    pub amp_usdc: Addr,
    pub arb_whale: Addr,
    pub amp_ash: Addr,
}
impl VaultAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<VaultAddrs, MigalooDestinationError> {
        Ok(VaultAddrs {
            amp_usdc: api.addr_validate(&self.amp_usdc)?,
            arb_whale: api.addr_validate(&self.arb_whale)?,
            amp_ash: api.addr_validate(&self.amp_ash)?,
        })
    }
}

/*
#[cw_serde]
pub struct WhaleLsdAddresses {
    /// migaloo1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqdhts4u
    pub bone_whale: String,
    /// migaloo1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgshqdky4
    pub amp_whale: String,
}
#[cw_serde]
pub struct WhaleLsdAddrs {
    pub bone_whale: Addr,
    pub amp_whale: Addr,
}
impl WhaleLsdAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<WhaleLsdAddrs, MigalooDestinationError> {
        Ok(WhaleLsdAddrs {
            bone_whale: api.addr_validate(&self.bone_whale)?,
            amp_whale: api.addr_validate(&self.amp_whale)?,
        })
    }
}*/

#[cw_serde]
#[derive(Default)]
pub struct SatelliteMarketAddresses {
    // migaloo1692nylpkryu7q00eukt93egtqu657z33nf0tedp0ps6htm8aty6qjdlpvh
    pub market: String,
    pub rewards: String,
}
#[cw_serde]
pub struct SatelliteMarketAddrs {
    pub market: Addr,
    pub rewards: Addr,
}
impl SatelliteMarketAddresses {
    pub fn validate_addrs(
        &self,
        api: &dyn Api,
    ) -> Result<SatelliteMarketAddrs, MigalooDestinationError> {
        Ok(SatelliteMarketAddrs {
            market: api.addr_validate(&self.market)?,
            rewards: api.addr_validate(&self.rewards)?,
        })
    }
}

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
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<DaoAddr, MigalooDestinationError> {
        Ok(DaoAddr {
            denom: self.denom.clone(),
            staking_address: api.addr_validate(&self.staking_address)?,
        })
    }
}
// impl DaoDaoAddresses {
//     pub fn validate_addrs(&self, api: &dyn Api) -> Result<DaoDaoAddrs, MigalooDestinationError> {
//         Ok(DaoDaoAddrs {
//             racoon_supply_dao: DaoAddr {
//                 denom: self.racoon_supply_dao.denom.clone(),
//                 staking_address: api.addr_validate(&self.racoon_supply_dao.staking_address)?,
//             },
//             guppy_dao: DaoAddr {
//                 denom: self.guppy_dao.denom.clone(),
//                 staking_address: api.addr_validate(&self.guppy_dao.staking_address)?,
//             },
//         })
//     }
// }

#[cw_serde]
#[derive(Default)]
pub struct DaoDaoAddresses {
    /// migaloo1398xz7e3zrv9ryx79lu8z46l3rqzy7tgta7dvhfx6853g8mp7fls43t7yf
    pub racoon_supply_dao: DaoAddress,
    /// migaloo1w3n0kcmrtwnj8dj6t6p7gm9szv0nsamezqw58374zz06jvstvr9q539fjw
    pub guppy_dao: DaoAddress,
}
#[cw_serde]
pub struct DaoDaoAddrs {
    pub racoon_supply_dao: DaoAddr,
    pub guppy_dao: DaoAddr,
}
impl DaoDaoAddresses {
    pub fn validate_addrs(&self, api: &dyn Api) -> Result<DaoDaoAddrs, MigalooDestinationError> {
        Ok(DaoDaoAddrs {
            racoon_supply_dao: self.racoon_supply_dao.validate_addrs(api)?,
            guppy_dao: self.guppy_dao.validate_addrs(api)?,
        })
    }
}

#[cw_serde]
pub enum GinkouExecuteMsg {
    DepositStable {},
}

#[cw_serde]
pub enum GinkouQueryMsg {
    EpochState {},
}

#[cw_serde]
pub struct GinkouEpochState {
    pub exchange_rate: Decimal,
    pub aterra_supply: Uint128,
    pub reserves_rate_used_for_borrowers: Decimal,
    pub prev_borrower_incentives: Uint128,
    pub last_interest_updated: Uint64,
}

#[cw_serde]
pub enum ErisMsg {
    Deposit {
        asset: Asset,
        receiver: Option<String>,
    },
    Bond {
        receiver: Option<String>,
    },
}
