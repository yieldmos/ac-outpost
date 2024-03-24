use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub enum MembraneStabilityPoolExecuteMsg {
    /// Deposit the debt token into the pool
    Deposit {
        /// User address, defaults to info.sender
        user: Option<String>,
    },
    /// Claim ALL liquidation revenue && MBRN incentives
    /// Can be queried from UserClaims
    ClaimRewards {},
}

#[cw_serde]
pub enum MembraneStakingExecuteMsg {
    Stake { user: Option<String> },
}

#[cw_serde]
pub enum MembraneCDPExecuteMsg {
    Deposit {
        /// Position ID to deposit into.
        /// If the user wants to create a new/separate position, no position id is passed.
        position_id: Option<Uint128>,
        /// Position owner.
        /// Defaults to the sender.
        position_owner: Option<String>,
    },
    // CDP Mint CDT
    IncreaseDebt {
        /// Position ID to increase debt of
        position_id: Uint128,
        /// Amount of debt to increase
        amount: Option<Uint128>,
        /// LTV to borrow up to
        LTV: Option<Decimal>,
        /// Mint debt tokens to this address
        mint_to_addr: Option<String>,
    },
    // CDP Repay position debt
    Repay {
        /// Position ID to repay debt of
        position_id: Uint128,
        /// Position owner to repay debt of if not the sender
        position_owner: Option<String>,
        /// Send excess assets to this address if not the sender
        send_excess_to: Option<String>,
    },
}

#[cw_serde]
pub enum MembraneCDPQueryMsg {
    /// Returns the contract's Basket
    GetBasket {},
}
