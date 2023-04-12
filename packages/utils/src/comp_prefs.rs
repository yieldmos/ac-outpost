use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use wyndex::asset::AssetInfo;

use crate::errors::OutpostError;

#[cw_serde]
pub struct CompoundPrefs {
    pub relative: Vec<DestinationAction>,
}

#[cw_serde]
pub struct DestinationAction {
    pub destination: JunoDestinationProject,
    /// the percentage of the rewards that should be sent to this destination
    /// this is a number with 18 decimal places
    /// for example "250000000000000000" is 25%
    pub amount: u128,
}

#[cw_serde]
pub enum JunoDestinationProject {
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
    pub comp_prefs: CompoundPrefs,
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

impl From<DestinationAction> for PoolCatchAllDestinationAction {
    fn from(
        DestinationAction {
            destination,
            amount,
        }: DestinationAction,
    ) -> Self {
        PoolCatchAllDestinationAction {
            destination: PoolCatchAllDestinationProject::BasicDestination(destination),
            amount,
        }
    }
}

impl From<CompoundPrefs> for Vec<PoolCatchAllDestinationAction> {
    fn from(CompoundPrefs { relative }: CompoundPrefs) -> Self {
        relative
            .into_iter()
            .map(PoolCatchAllDestinationAction::from)
            .collect()
    }
}
