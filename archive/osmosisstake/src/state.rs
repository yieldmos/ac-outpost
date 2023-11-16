use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::OutpostAddresses;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const AUTHORIZED_ADDRS: Item<Vec<Addr>> = Item::new("allowed_addrs");
pub const OUTPOST_ADDRS: Item<OutpostAddresses> = Item::new("outpost_addrs");
