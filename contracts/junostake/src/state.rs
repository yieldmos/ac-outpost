use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ContractAddresses;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const AUTHORIZED_ADDRS: Item<Vec<Addr>> = Item::new("allowed_addrs");
pub const PROJECT_ADDRS: Item<ContractAddresses> = Item::new("project_addrs");
