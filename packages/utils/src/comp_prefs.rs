use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use wyndex::asset::AssetInfo;

#[cw_serde]
pub struct CompoundPrefs {
    pub relative: Vec<DestinationAction>,
}

#[cw_serde]
pub struct DestinationAction {
    pub destination: DestinationProject,
    pub amount: u128,
}

#[cw_serde]
pub enum DestinationProject {
    JunoStaking {
        validator_address: String,
    },
    WyndStaking {
        bonding_period: WyndStakingBondingPeriod,
    },
    WyndLP {
        contract_address: String,
        bonding_period: WyndLPBondingPeriod,
    },
    TokenSwap {
        target_denom: AssetInfo,
    },
    NetaStaking {},
}

#[cw_serde]
pub struct ValidatorSelection {
    pub validator_address: String,
    pub percent: Decimal,
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
