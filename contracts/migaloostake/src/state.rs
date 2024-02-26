use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ContractAddrs;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const AUTHORIZED_ADDRS: Item<Vec<Addr>> = Item::new("allowed_addrs");
pub const PROJECT_ADDRS: Item<ContractAddrs> = Item::new("project_addrs");
