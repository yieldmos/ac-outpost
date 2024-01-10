use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

/// Polyfilled FundMsg from SparkIBC
#[cw_serde]
pub enum FundMsg {
    FundGeneral {
        donor_address_type: AddressType,
        on_behalf_of: Option<String>,
    },
    FundCampaign {
        campaign_name: String,
        donor_address_type: AddressType,
        on_behalf_of: Option<String>,
    },
}

#[cw_serde]
pub enum SparkIbcFund {
    Fund(FundMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub enum AddressType {
    Private,
    Validator,
    Organization,
}

#[cw_serde]
pub enum LsdQueryMsg {
    State {},
}

#[cw_serde]
pub struct LsdStateResponse {
    /// Total supply to the Stake token
    pub total_ustake: Uint128,
    /// Total amount of utoken staked (bonded)
    pub total_utoken: Uint128,
    /// The exchange rate between ustake and utoken, in terms of utoken per ustake
    pub exchange_rate: Decimal,
    /// Staking rewards currently held by the contract that are ready to be reinvested
    pub unlocked_coins: Vec<Coin>,
    // Amount of utoken currently unbonding
    pub unbonding: Uint128,
    // Amount of utoken currently available as balance of the contract
    pub available: Uint128,
    // Total amount of utoken within the contract (bonded + unbonding + available)
    pub tvl_utoken: Uint128,
}
