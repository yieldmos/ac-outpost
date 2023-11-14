use std::collections::HashMap;

use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{Addr, Uint128};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
