use crate::errors::OsmosisDestinationError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Event, Uint128};
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};

pub type DestinationResult = Result<DestProjectMsgs, OsmosisDestinationError>;

pub fn mint_milk_tia_msgs(
    minter_addr: &Addr,
    milk_tia_addr: &Addr,
    tia_to_bond: Uint128,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            milk_tia_addr,
            minter_addr,
            &MilkyWayExecuteMsg::LiquidStake {},
            None,
        )?)],
        sub_msgs: vec![],
        events: vec![
            Event::new("mint_milk_tia").add_attribute("tia_to_bond", tia_to_bond.to_string())
        ],
    })
}

#[cw_serde]
pub enum MilkyWayExecuteMsg {
    LiquidStake {},
}
