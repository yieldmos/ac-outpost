use crate::errors::OsmosisDestinationError;

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Event, Uint128};
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};

pub type DestinationResult = Result<DestProjectMsgs, OsmosisDestinationError>;

pub fn mint_milk_tia_msgs(
    minter_addr: &Addr,
    milk_tia_addr: &Addr,
    tia_to_bond: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            milk_tia_addr,
            minter_addr,
            &MilkyWayExecuteMsg::LiquidStake {},
            Some(vec![CsdkCoin {
                denom: tia_to_bond.denom.to_string(),
                amount: tia_to_bond.amount.to_string(),
            }]),
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

pub fn stake_ion_msgs(
    staker_addr: &Addr,
    ion_dao_addr: &Addr,
    ion_to_stake: Uint128,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            ion_dao_addr,
            staker_addr,
            &cw20_stake::msg::ReceiveMsg::Stake {},
            Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: "uion".to_string(),
                amount: ion_dao_addr.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("stake_ion").add_attribute("amount", ion_to_stake.to_string())],
    })
}
