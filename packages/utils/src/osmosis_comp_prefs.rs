use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

use crate::comp_prefs::CompoundPrefs;

pub type OsmosisCompPrefs = CompoundPrefs<OsmosisDestinationProject>;

#[cw_serde]
pub enum OsmosisDestinationProject {
    OsmosisStaking { validator_address: String },
    TokenSwap { target_denom: Coin },
    OsmosisLiquidityPool { pool_id: u64 },
    // RedBankLending { asset: Coin },
    // RedBankVault {
    //     vault_address: String,
    //     leverage_amount: u64,

    // },
}
