use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use osmosis_destinations::pools::{StoredDenoms, StoredPools};

use crate::msg::ContractAddrs;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const AUTHORIZED_ADDRS: Item<Vec<Addr>> = Item::new("allowed_addrs");
pub const PROJECT_ADDRS: Item<ContractAddrs> = Item::new("project_addrs");

pub const KNOWN_OSMO_POOLS: StoredPools = Map::new("known_osmo_pools");
pub const KNOWN_USDC_POOLS: StoredPools = Map::new("known_usdc_pools");
pub const KNOWN_DENOMS: StoredDenoms = Map::new("known_denoms");
