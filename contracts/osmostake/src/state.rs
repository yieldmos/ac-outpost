use crate::msg::ContractAddrs;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use osmosis_destinations::{
    comp_prefs::MembraneDepositCollateralAction,
    pools::{StoredDenoms, StoredPools},
};
use outpost_utils::comp_prefs::TakeRate;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const AUTHORIZED_ADDRS: Item<Vec<Addr>> = Item::new("allowed_addrs");
pub const TAKE_RATE: Item<TakeRate> = Item::new("take_rate");
pub const PROJECT_ADDRS: Item<ContractAddrs> = Item::new("project_addrs");

pub const KNOWN_OSMO_POOLS: StoredPools = Map::new("known_osmo_pools");
pub const KNOWN_USDC_POOLS: StoredPools = Map::new("known_usdc_pools");
pub const KNOWN_DENOMS: StoredDenoms = Map::new("known_denoms");

pub const SUBMSG_REPLY_ID: Item<u64> = Item::new("submsg_reply_id");
pub const SUBMSG_DATA: Map<&u64, SubmsgData> = Map::new("submsg_data");

#[cw_serde]
pub enum SubmsgData {
    MintCdt {
        user_addr: Addr,
        position_id: Uint128,
        and_then: MembraneDepositCollateralAction,
    },
    BondGamms {
        pool_id: u64,
    },
}
