use cosmwasm_schema::cw_serde;

use crate::comp_prefs::CompoundPrefs;

pub type OsmosisCompPrefs = CompoundPrefs<OsmosisDestinationProject>;

#[cw_serde]
pub enum RedBankLeverRewardTarget {
    LeaveLiquid,
    Reexpose,
    Repay,
}

#[cw_serde]
pub enum OsmosisDestinationProject {
    /// Stake the tokens to a given validator
    OsmosisStaking { validator_address: String },

    /// Swap the given denom for the target denom and leave that token liquid.
    TokenSwap { target_denom: String },
    /// Pay back borrowed balance. Currently the first denom strings specified in the vector will be
    /// paid back first. No order is guaranteed when no vector is passed in.
    /// Eventually there should be an option to pay back the highest cost debt first
    RedBankPayback(PaybackDenoms),

    /// Deposit into redbank to potentially gain
    RedBankDeposit {
        /// IMPORTANT: if the deposit cap is reached, the compounding will not be forced to
        /// error out. Instead, the alloted funds for depositing will remain liquid and unswapped and undeposited
        target_denom: String,
    },
    /// Continuously lever up the given denom
    RedBankLeverLoop {
        /// the denom to continuously lever up.
        /// at time of writing the options are atom, osmo, usdc, wbtc, weth
        denom: String,
        /// this is the percentage of the collateral that will be borrowed.
        /// should be a number with 18 places.
        /// defaults to 50%
        ltv_ratio: Option<u128>,
    },

    /// Convert to Ion and stake it
    IonStaking {},

    // Swap to the appropriate pool tokens, join the pool, and lock the tokens for 14 days if desired
    OsmosisLiquidityPool {
        pool_id: u64,
        // If true, the pool tokens will be locked with a 14 day unbonding period
        bond_tokens: bool,
    },
    // RedBankVault {
    //     vault_address: String,
    //     leverage_amount: u64,
    // },
}

#[cw_serde]
pub enum PaybackDenoms {
    /// Pay back the given denoms only
    Only(Vec<String>),
    /// If no denom is set then pay back loans indiscriminately otherwise start with the given denom and then move onto the others
    Any(Option<Vec<String>>),
}
