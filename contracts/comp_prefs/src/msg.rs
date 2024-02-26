use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint64;

use crate::state::{CompPref, StoreSettings, UnverifiedUserCompPref};

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract administrator-
    /// if not specified it will default to the contract instantiator
    pub admin: Option<String>,

    /// the chain id of the compounding prefs that we want to store
    /// on this contract
    pub chain_id: String,

    /// How long (in days) after a set of prefs goes inactive it is allowed to be pruned
    pub days_to_prune: u16,
}

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// For changing the contract's priviledged user
    /// ADMIN ONLY ACTION
    SetAdmin(String),

    /// Add a new allowed strat id - can be used when a new outpost is added
    /// ADMIN ONLY ACTION
    AddAllowedStrategyId(Uint64),

    /// Remove an existing strat id
    /// ADMIN ONLY ACTION
    RemoveAllowedStrategyId(Uint64),

    /// Store the settings that should be used for outpost compounding.
    /// This can be called for first time activation or for updates
    SetCompoundingPreferences(UnverifiedUserCompPref),

    /// Early cancellation of compounding preferences
    /// Takes the the strategy id that should be cancelled
    CancelCompoundingPreferences(Uint64),

    /// Updates compounding prefs to accurately store data on inactivity
    /// and removes settings that are more than `days_to_prune` days older than the
    /// time they became inactive
    PruneInactiveCompoundingPreferences { limit: u16, offset: u32 },
}

#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    /// Gets the global settings of the whole storage contract
    #[returns(StoreSettings)]
    StoreSettings,

    #[returns(Vec<Uint64>)]
    AllowedStrategyIds,

    /// Gets the strategy settings for a given user and strategyId
    #[returns(Option<CompPref>)]
    StrategyPreferencesByUserAndStratId {
        user_address: String,
        strategy_id: Uint64,
    },

    /// Gets all of the stored strategy settings for a given user
    #[returns(Vec<CompPref>)]
    StrategyPreferencesByUser {
        user_address: String,
        status: Option<CompPrefStatus>,
    },

    #[returns(Vec<CompPref>)]
    StrategyPreferencesByStratId {
        strat_id: Uint64,
        /// experimental status filter- maybe have unpredictable gas usage
        /// buyer beware
        status: Option<CompPrefStatus>,
        limit: Option<u16>,
        /// address to resume pagination from
        prev_address: Option<String>,
    },

    /// Gets all of the stored strategy settings for the user with
    /// the given pubkey
    #[returns(Vec<CompPref>)]
    StrategyPreferencesByPubkey {
        pubkey: String,
        status: Option<CompPrefStatus>,
    },
}

#[cw_serde]
pub enum CompPrefStatus {
    // Campaign is currently valid and not expired
    Active,
    // Either expired or cancelled
    Inactive,
    // Strategy ran it's whole course without being manually cancelled
    Expired,
    // Strategy was cancelled before expiration
    Cancelled,
}
