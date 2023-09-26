use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use wyndex::asset::AssetInfo;

use crate::{
    comp_prefs::{CompoundPrefs, DestinationAction},
    errors::OutpostError,
};

pub type JunoCompPrefs = CompoundPrefs<JunoDestinationProject>;

#[cw_serde]
pub enum JunoDestinationProject {
    /// Native Staking on juno
    JunoStaking { validator_address: String },
    /// Send tokens to a specific address
    SendTokens { denom: AssetInfo, address: String },
    /// Swapping to an abitrary token via Wyndex
    /// https://app.wynddao.com/swap
    TokenSwap { target_denom: AssetInfo },
    /// Staking to WyndDAO
    /// https://app.wynddao.com/stake
    WyndStaking {
        bonding_period: WyndStakingBondingPeriod,
    },
    /// Joining any Wyndex LP
    /// https://app.wynddao.com/pools
    WyndLp {
        contract_address: String,
        bonding_period: WyndLPBondingPeriod,
    },
    /// Gelotto reoccuring lotteries
    /// https://gelotto.io/app/games/max
    GelottoLottery {
        lottery: GelottoLottery,
        lucky_phrase: u32,
    },
    /// Spark IBC Campaign Funding
    /// https://sparkibc.zone/earn
    SparkIbcCampaign { fund: FundMsg },
    /// Swap on BalanceDao
    /// https://www.balancedao.zone/
    BalanceDao {},
    /// Join the White Whale satellite market
    /// https://app.whitewhale.money/juno/dashboard
    // WhiteWhaleSatellite { asset: pool_network::asset::Asset },
    WhiteWhaleSatellite { asset: AssetInfo },
    /// Swap token and stake to the specified daodao
    /// https://daodao.zone/
    DaoStaking(StakingDao),
    /// Table Games on RacoonBet
    /// https://www.racoon.bet/
    RacoonBet { game: RacoonBetGame },
    /// Mint one of many Juno LSDs
    MintLsd { lsd_type: JunoLsd },
    /// Do nothing with the funds
    Unallocated {},
}

#[cw_serde]
pub enum StakingDao {
    /// Neta Dao
    /// https://daodao.zone/dao/juno1c5v6jkmre5xa9vf9aas6yxewc7aqmjy0rlkkyk4d88pnwuhclyhsrhhns6
    Neta,
    /// Signal Dao
    /// https://daodao.zone/dao/juno1tr4t593vy37qtqqh28tarmj34yae9za9zlj7xeegx3k8rgvp3xeqv02tu5
    Signal,
    /// Posthuman Dao
    /// https://daodao.zone/dao/juno1h5ex5dn62arjwvwkh88r475dap8qppmmec4sgxzmtdn5tnmke3lqwpplgg
    Posthuman,
    /// Kleomedes Dao
    /// https://daodao.zone/dao/juno1mue2xdl05375tjc4njux5c6mkxltun3h0p33qtpx4utrwtnh949sxutcxy
    Kleomedes,
    /// CannaLabs Dao
    /// https://daodao.zone/dao/juno17wu0h9sypnrfuk7x48ptsqrdqljcen92dwracqgzs5dl6p0n0jfs4qzj82
    CannaLabs,
    // /// Casa Dao
    // /// https://daodao.zone/dao/juno1dnphxeyxcewgj9ta0ht2cywxu6f43wx2zevcum2mzjvwvt89aa8sredp6d
    // Casa,
    /// Muse Dao
    /// https://daodao.zone/dao/juno14k5fhw33e3dulvcfrq7d5sdfdwzhcxnecmptsv4x5lzyc67ne46qxt4x8y
    Muse,
}
impl std::fmt::Display for StakingDao {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            StakingDao::Neta => write!(f, "Neta Dao"),
            StakingDao::Signal => write!(f, "Signal Dao"),
            StakingDao::Posthuman => write!(f, "Posthuman Dao"),
            StakingDao::Kleomedes => write!(f, "Kleomedes Dao"),
            StakingDao::CannaLabs => write!(f, "CannaLabs Dao"),
            StakingDao::Muse => write!(f, "Muse Dao"),
        }
    }
}
impl StakingDao {
    pub fn get_daos_addresses(&self, addresses: &DaoAddresses) -> DaoAddress {
        match self {
            StakingDao::Neta => addresses.neta.clone(),
            StakingDao::Signal => addresses.signal.clone(),
            StakingDao::Posthuman => addresses.posthuman.clone(),
            StakingDao::Kleomedes => addresses.kleomedes.clone(),
            StakingDao::CannaLabs => addresses.cannalabs.clone(),
            StakingDao::Muse => addresses.muse.clone(),
        }
    }
}

#[cw_serde]
pub enum JunoLsd {
    /// Mint Backbone's liquid staked Juno token (bJUNO)
    /// https://juno.gravedigger.zone/
    Backbone,
    /// Mint Juno LSD on WyndDAO (wyJUNO)
    /// https://app.wynddao.com/lsd/1
    Wynd,
    /// SE variant of stakeeasy's juno lsd (seJUNO)
    /// https://juno.stakeeasy.finance/
    StakeEasySe,
    /// B variant of stakeeasy's juno lsd (bJUNO)
    /// https://juno.stakeeasy.finance/
    StakeEasyB,
    /// Mint Juno LSD on Eris Protocol (ampJUNO)
    /// https://www.erisprotocol.com/juno/amplifier
    Eris,
}
impl JunoLsd {
    pub fn get_mint_address(&self, addresses: &JunoLsdAddresses) -> String {
        match self {
            JunoLsd::Backbone => addresses.amp_juno.clone(),
            JunoLsd::Wynd => addresses.wy_juno.clone(),
            JunoLsd::StakeEasySe => addresses.se_juno.clone(),
            JunoLsd::StakeEasyB => addresses.b_juno.clone(),
            JunoLsd::Eris => addresses.amp_juno.clone(),
        }
    }
}

impl std::fmt::Display for JunoLsd {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            JunoLsd::Backbone => write!(f, "boneJUNO"),
            JunoLsd::Wynd => write!(f, "wyJUNO"),
            JunoLsd::StakeEasyB => write!(f, "bJUNO"),
            JunoLsd::StakeEasySe => write!(f, "seJUNO"),
            JunoLsd::Eris => write!(f, "ampJUNO"),
        }
    }
}

#[cw_serde]
pub enum RacoonBetGame {
    HundredSidedDice {
        /// 1-100
        /// The odds of winning, the lower the odds the higher the prize
        selected_value: u8,
    },
    Slot {
        /// 1-100 based on user input. Represents the number of slot plays to do. Each spin must be worth at least 1 USD
        spins: u32,
        /// the token amount (of the token being wagered) to use for each spin. the total amount wagered must be at least 1 USD
        spin_value: Uint128,
        /// Should pass 0 for the outpost usage to leave empowered spins for the users to enjoy manually
        empowered: Uint128,
        /// Should pass 0 for the outpost usage to leave free spins for the users to enjoy manually
        free_spins: Uint128,
    },
}

impl std::fmt::Display for RacoonBetGame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            RacoonBetGame::HundredSidedDice { selected_value } => {
                write!(f, "racoon dice, value: {}", selected_value)
            }
            RacoonBetGame::Slot {
                spins,
                spin_value,
                empowered,
                free_spins,
            } => write!(
                f,
                "racoon slots, spins: {}, spin_value: {}, empowered: {}, free_spins: {}",
                spins, spin_value, empowered, free_spins
            ),
        }
    }
}

#[cw_serde]
pub enum RacoonBetExec {
    PlaceBet { game: RacoonBetGame },
}

#[cw_serde]
pub enum GelottoLottery {
    Pick3,
    Pick4,
    Pick5,
}
// implement Display for gelotto lottery
impl std::fmt::Display for GelottoLottery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GelottoLottery::Pick3 => write!(f, "Pick 3"),
            GelottoLottery::Pick4 => write!(f, "Pick 4"),
            GelottoLottery::Pick5 => write!(f, "Pick 5"),
        }
    }
}

/// Polyfilled FundMsg from SparkIBC
#[cw_serde]
pub enum FundMsg {
    FundGeneral {
        donor_address_type: AddressType,
        on_behalf_of: String,
    },
    FundCampaign {
        campaign_name: String,
        donor_address_type: AddressType,
        on_behalf_of: String,
    },
}

#[cw_serde]
pub enum SparkIbcFund {
    Fund(FundMsg),
}

#[cw_serde]
pub enum AddressType {
    Private,
    Validator,
}

#[cw_serde]
pub enum WyndStakingBondingPeriod {
    ThirtyDays = 2592000,
    NinetyDays = 7776000,
    OneHundredEightyDays = 15552000,
    ThreeHundredSixtyFiveDays = 31536000,
    SevenHundredThirtyDays = 63072000,
}

impl From<WyndStakingBondingPeriod> for u64 {
    fn from(v: WyndStakingBondingPeriod) -> Self {
        match v {
            WyndStakingBondingPeriod::ThirtyDays => 2592000,
            WyndStakingBondingPeriod::NinetyDays => 7776000,
            WyndStakingBondingPeriod::OneHundredEightyDays => 15552000,
            WyndStakingBondingPeriod::ThreeHundredSixtyFiveDays => 31536000,
            WyndStakingBondingPeriod::SevenHundredThirtyDays => 63072000,
        }
    }
}

#[cw_serde]
pub enum WyndLPBondingPeriod {
    SevenDays = 604800,
    FourteenDays = 1209600,
    TwentyEightDays = 2419200,
    FourtyTwoDays = 3628800,
}

impl From<WyndLPBondingPeriod> for u64 {
    fn from(v: WyndLPBondingPeriod) -> Self {
        match v {
            WyndLPBondingPeriod::SevenDays => 604800,
            WyndLPBondingPeriod::FourteenDays => 1209600,
            WyndLPBondingPeriod::TwentyEightDays => 2419200,
            WyndLPBondingPeriod::FourtyTwoDays => 3628800,
        }
    }
}
// implement try_from for u64 to WyndLPBondingPeriod
impl TryFrom<u64> for WyndLPBondingPeriod {
    type Error = OutpostError;

    fn try_from(v: u64) -> Result<Self, Self::Error> {
        match v {
            604800 => Ok(WyndLPBondingPeriod::SevenDays),
            1209600 => Ok(WyndLPBondingPeriod::FourteenDays),
            2419200 => Ok(WyndLPBondingPeriod::TwentyEightDays),
            3628800 => Ok(WyndLPBondingPeriod::FourtyTwoDays),
            _ => Err(OutpostError::InvalidBondingPeriod(v.to_string())),
        }
    }
}

#[cw_serde]
/// compound prefs for a specific pool
pub struct PoolCompoundPrefs {
    pub pool_address: String,
    pub comp_prefs: CompoundPrefs<JunoDestinationProject>,
}

#[cw_serde]
/// compound prefs for all of the pools that have rewards and were not
/// individually specified
pub struct PoolCatchAllDestinationAction {
    pub destination: PoolCatchAllDestinationProject,
    /// the percentage of the rewards that should be sent to this destination
    /// this is a number with 18 decimal places
    /// for example "250000000000000000" is 25%
    pub amount: u128,
}

#[cw_serde]
/// Compound prefs for a catch all pools that were not individually specified.
/// The main difference between this and the normal DestinationProject is that
/// in the catch all you have the ability to specify sending the rewards back to the pool
/// it came from instead of needing to specify any static destination
pub enum PoolCatchAllDestinationProject {
    BasicDestination(JunoDestinationProject),
    /// send pool rewards back to the pool that generated the rewards
    ReturnToPool,
}

impl From<DestinationAction<JunoDestinationProject>> for PoolCatchAllDestinationAction {
    fn from(
        DestinationAction {
            destination,
            amount,
        }: DestinationAction<JunoDestinationProject>,
    ) -> Self {
        PoolCatchAllDestinationAction {
            destination: PoolCatchAllDestinationProject::BasicDestination(destination),
            amount,
        }
    }
}

impl From<CompoundPrefs<JunoDestinationProject>> for Vec<PoolCatchAllDestinationAction> {
    fn from(CompoundPrefs { relative }: CompoundPrefs<JunoDestinationProject>) -> Self {
        relative
            .into_iter()
            .map(PoolCatchAllDestinationAction::from)
            .collect()
    }
}

#[cw_serde]
pub struct DestinationProjectAddresses {
    pub wynd: WyndAddresses,
    pub gelotto: GelottoAddresses,
    pub daos: DaoAddresses,
    pub spark_ibc: SparkIbcAddresses,
    // juno1ve7y09kvvnjk0yc2ycaq0y9thq5tct5ve6c0a5hfkt0h4jfy936qxtne5s
    pub balance_dao: String,
    pub white_whale: WhiteWhaleSatelliteAddresses,
    pub racoon_bet: RacoonBetAddresses,
    pub juno_lsds: JunoLsdAddresses,
}

#[cw_serde]
pub struct RacoonBetAddresses {
    // juno1h8p0jmfn06nfqpn0medn698h950vnl7v54m2azkyjdqjlzcly7jszxh7yu
    pub game: String,
    // juno1gqy6rzary8vwnslmdavqre6jdhakcd4n2z4r803ajjmdq08r66hq7zcwrj
    pub juno_usdc_wynd_pair: String,
}

#[cw_serde]
pub struct SparkIbcAddresses {
    // juno1a6rna5tcl6p97rze6hnd5ug35kadqhudvr5f4mtr6s0yd5mruhss8gzrdy
    pub fund: String,
}

#[cw_serde]
pub struct JunoLsdAddresses {
    // juno102at0mu2xeluyw9efg257yy6pyhv088qqhmp4f8wszqcwxnpdcgqsfq0nv
    pub bone_juno: String,
    // juno18wuy5qr2mswgz7zak8yr9crhwhtur3v6mw4tcytupywxzw7sufyqgza7uh
    pub wy_juno: String,
    // juno1dd0k0um5rqncfueza62w9sentdfh3ec4nw4aq4lk5hkjl63vljqscth9gv
    pub se_juno: String,
    // juno1wwnhkagvcd3tjz6f8vsdsw5plqnw8qy2aj3rrhqr2axvktzv9q2qz8jxn3
    pub b_juno: String,
    //
    pub amp_juno: String,
}

#[cw_serde]
pub struct WhiteWhaleSatelliteAddresses {
    // ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF
    pub amp_whale: AssetInfo,
    // ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF
    pub bone_whale: AssetInfo,

    pub juno_amp_whale_path: white_whale::pool_network::router::SwapRoute,
    pub juno_bone_whale_path: white_whale::pool_network::router::SwapRoute,

    // The contract address for the multihop router
    // juno128lewlw6kv223uw4yzdffl8rnh3k9qs8vrf6kef28579w8ygccyq7m90n2
    pub terraswap_multihop_router: String,

    /// The contract address for bonding to the satellite market
    // juno1n8slcc79dmwuzdxhsesvhcncaqfg9h4czdm5t5ey8x25ajmn3xzqyde4wv
    pub market: String,

    /// The contract address for claiming the satellite market rewards
    // juno184ghwgprva7dlr2hwhzgvt6mem6zx78fygk0cpw7klssmzyf67tqdtwt3h
    pub rewards: String,
}

#[cw_serde]
pub struct DaoAddresses {
    // cw20: juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr
    // staking: juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh
    // juno_wyndex_pair: juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra
    // wynd_wyndex_pair:
    pub neta: DaoAddress,

    // cw20: juno14lycavan8gvpjn97aapzvwmsj8kyrvf644p05r0hu79namyj3ens87650k
    // juno_wyndex_pair: juno1p3eed298qx3nyhs3grld07jrf9vjsjsmdd2kmmh3crk87emjcx5stp409y
    // wynd_wyndex_pair:
    pub signal: DaoAddress,

    // cw20: juno1rws84uz7969aaa7pej303udhlkt3j9ca0l3egpcae98jwak9quzq8szn2l
    // juno_wyndex_pair: juno17jv00cm4f3twr548jzayu57g9txvd4zdh54mdg9qpjs7samlphjsykylsq
    // wynd_wyndex_pair:
    pub posthuman: DaoAddress,

    // cw20: juno10gthz5ufgrpuk5cscve2f0hjp56wgp90psqxcrqlg4m9mcu9dh8q4864xy
    // juno_wyndex_pair: juno1dpqgt3ja2kdxs94ltjw9ncdsexts9e3dx5qpnl20zvgdguzjelhqstf8zg
    // wynd_wyndex_pair:
    pub kleomedes: DaoAddress,

    // cw20: juno1vn38rzq0wc7zczp4dhy0h5y5kxh2jjzeahwe30c9cc6dw3lkyk5qn5rmfa
    // juno_wyndex_pair: juno17ckp36lmgtt7jtuggdv2j39eh4alcnl35szu6quh747nujags07swwq0nh
    // wynd_wyndex_pair: juno1ls5un4a8zyn4f05k0ekq5aa9uhn88y8362ww38elqfpcwllme0jqelamke
    pub cannalabs: DaoAddress,

    // cw20: juno1p8x807f6h222ur0vssqy3qk6mcpa40gw2pchquz5atl935t7kvyq894ne3
    // juno_wyndex_pair: juno1rcssjyqgr6vzalss77d43v30c2qpyzzg607ua8gte2shqgtvu24sg8gs8r
    // wynd_wyndex_pair:
    pub muse: DaoAddress,
}

#[cw_serde]
pub struct WyndAddresses {
    // juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9
    pub cw20: String,
    // juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w
    pub multihop: String,
    // juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf
    pub juno_wynd_pair: String,
}

#[cw_serde]
pub struct DaoAddress {
    pub cw20: String,
    pub staking: String,
    pub juno_wyndex_pair: Option<String>,
    pub wynd_wyndex_pair: Option<String>,
}

#[cw_serde]
pub struct GelottoAddresses {
    pub pick3_contract: String,
    pub pick4_contract: String,
    pub pick5_contract: String,
}
impl GelottoLottery {
    pub fn get_lottery_address(&self, addresses: &GelottoAddresses) -> String {
        match self {
            GelottoLottery::Pick3 => addresses.pick3_contract.clone(),
            GelottoLottery::Pick4 => addresses.pick4_contract.clone(),
            GelottoLottery::Pick5 => addresses.pick5_contract.clone(),
        }
    }
}

#[cw_serde]
pub struct Bond {}

#[cw_serde]
pub enum GelottoExecute {
    SenderBuySeed {
        referrer: Option<Addr>,
        count: u16,
        seed: u32,
    },
}
