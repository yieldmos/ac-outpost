use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Timestamp, Uint64};
use cw_storage_plus::{Item, Map};

// General settings for the whole contract
pub const STORE_SETTINGS: Item<StoreSettings> = Item::new("store_settings");

// Map of all the strategy ids that are allowed.
pub const ALLOWED_STRATEGY_IDS: Map<u64, ()> = Map::new("allowed_strategy_ids");

// Tuple of strat id and user address  that matches with that user's compounding preferences
pub const COMP_PREFS: Map<(u64, &Addr), CompPref> = Map::new("compounding_preferences");

// structure for querying by pubkey.
// returns the compound key that will be able to get the user's preferences from
// the COMP_PREFS map
// note: this doesnt actually use a true pubkey but should be a unique identifier for the wallet
pub const PREFS_BY_PUBKEY: Map<(&str, u64, Addr), ()> = Map::new("comp_prefs_by_pubkey");

#[cw_serde]
pub struct UnverifiedUserCompPref {
    /// address of the outpost contract
    pub outpost_address: String,

    /// Address of the user account that is having their strategy run
    pub address: String,

    /// Strategy Id that the settings apply to
    pub strat_id: Uint64,

    /// base64 encoded json that describes the setting that are expected
    /// to be used for the account's compounding
    pub strategy_settings: Binary,

    /// Frequency that the compounding should occur at
    pub comp_period: CompoundingFrequency,

    /// Unique identifier for indexing associated addresses cross-chain
    pub pub_key: String,

    /// Marks the ending timestamp of the grants/strategy that were initially set
    pub expires: Timestamp,
}

#[cw_serde]
pub struct UserCompPref {
    /// address of the outpost contract
    pub outpost_address: Addr,

    /// Strategy Id that the settings apply to
    pub strat_id: u64,

    /// base64 encoded json that describes the setting that are expected
    /// to be used for the account's compounding
    pub strategy_settings: Binary,

    /// Address of the user account that is having their strategy run
    pub address: Addr,

    /// Frequency that the compounding should occur at
    pub comp_period: CompoundingFrequency,

    /// Unique identifier for indexing associated addresses cross-chain
    pub pub_key: String,

    /// Marks the ending timestamp of the grants/strategy that were initially set
    pub expires: Timestamp,
}

#[cw_serde]
pub struct InactiveStatus {
    /// Explanation as to why the comp prefs became inactive
    pub end_type: EndType,
    /// When the prefs became inactive (not necessarily when they were moved into the ended queue)
    pub ended_at: Timestamp,
}

#[cw_serde]
pub enum EndType {
    /// The prefs were explicitly cancelled before their end date
    Cancellation,

    /// The prefs expired and were not renewed
    Expiration,
}

#[cw_serde]
pub struct CompPref {
    pub user_comp_pref: UserCompPref,

    /// chain id that the prefs apply to
    pub chain_id: String,

    /// The timestamp of the first time the user set comp prefs for this user address and strat id
    pub created_at: Timestamp,

    /// The last time the prefs were updated or reactivated.
    /// If never updated or reactivated this should stay as the same timestamp as `created_at`
    pub updated_at: Timestamp,

    /// Information about the transition of the strategy to an inactive state
    /// Can be falsely `None` in some cases due to strategy expirations passing
    pub is_inactive: Option<InactiveStatus>,
}

#[cw_serde]
pub enum CompoundingFrequency {
    Hourly = 3600,
    TwoTimesADay = 43200,
    Daily = 86400,
    Weekly = 604800,
    Monthly = 2629746,
    Quarterly = 7889400,
}

#[cw_serde]
pub struct StoreSettings {
    pub admin: Addr,
    pub chain_id: String,

    /// How long (in days) after a set of prefs goes inactive it is allowed to be pruned
    pub days_to_prune: u16,
}
