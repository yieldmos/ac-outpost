use cosmwasm_schema::cw_serde;

use crate::comp_prefs::CompoundPrefs;

pub type OsmosisCompPrefs = CompoundPrefs<OsmosisDestinationProject>;

#[cw_serde]
pub enum OsmosisDestinationProject {
    OsmosisStaking {
        validator_address: String,
    },
    TokenSwap {
        target_denom: String,
    },
    RedBankDeposit {
        /// IMPORTANT: if the deposit cap is reached, the compounding will not be forced to
        /// error out. Instead, the alloted funds for depositing will remain liquid and unswapped and undeposited
        target_denom: String,
    },
    // OsmosisLiquidityPool { pool_id: u64 },
    // RedBankVault {
    //     vault_address: String,
    //     leverage_amount: u64,

    // },
}
