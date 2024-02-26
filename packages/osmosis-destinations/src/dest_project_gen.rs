use crate::{
    errors::OsmosisDestinationError,
    mars_types::{RedBankAction, RedBankExecuteMsgs},
};

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

// fund account and lend the asset if wanted
pub fn fund_red_bank_acct_msgs(
    funder_addr: &Addr,
    funder_account_id: &str,
    redbank_addr: &Addr,
    fund_amount: Coin,
    lend_asset: bool,
) -> DestinationResult {
    // fund the account
    let mut actions: Vec<RedBankAction> = vec![RedBankAction::Deposit(fund_amount.clone())];

    // if the user wants to lend the asset add that action to the end
    if lend_asset {
        actions.push(RedBankAction::Lend((&fund_amount).into()));
    }

    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            redbank_addr,
            funder_addr,
            &RedBankExecuteMsgs::UpdateCreditAccount {
                account_id: funder_account_id.to_string(),
                actions,
            },
            Some(vec![CsdkCoin {
                denom: fund_amount.denom.to_string(),
                amount: fund_amount.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("fund_red_bank_acct")
            .add_attribute("fund_amount", fund_amount.to_string())
            .add_attribute("fund_and_lend", lend_asset.to_string())],
    })
}

// stake mbrn
pub fn stake_mbrn_msgs(
    staker_addr: &Addr,
    membrane_staking_contract_addr: &Addr,
    mbrn_to_stake: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            membrane_staking_contract_addr,
            staker_addr,
            &membrane::staking::ExecuteMsg::Stake { user: None },
            Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: mbrn_to_stake.denom.to_string(),
                amount: mbrn_to_stake.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("stake_mbrn").add_attribute("amount", mbrn_to_stake.to_string())],
    })
}
