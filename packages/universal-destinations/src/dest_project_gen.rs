use cosmos_sdk_proto::cosmos::{
    bank::v1beta1::MsgSend, base::v1beta1::Coin as CsdkCoin, staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{to_json_binary, Coin, Event, Uint128};
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};

use std::fmt::Display;
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::errors::UniversalDestinationError;

pub type DestinationResult = Result<DestProjectMsgs, UniversalDestinationError>;

/// Generates the messages needed to delegate to a validator
pub fn native_staking_msg(
    validator_address: &str,
    delegator_addr: &(impl Into<String> + Display),
    coin: &Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::Delegate(MsgDelegate {
            validator_address: validator_address.to_string(),
            amount: Some(CsdkCoin {
                denom: coin.denom.clone(),
                amount: coin.amount.into(),
            }),
            delegator_address: delegator_addr.to_string(),
        })],
        events: vec![Event::new("delegate")
            .add_attribute("validator", validator_address)
            .add_attribute("amount", coin.to_string())],
    })
}

// stakes an asset to a dao on daodao
pub fn daodao_staking_msg<T>(
    dao_name: String,
    staker_address: &T,
    staking_contract_addr: &T,
    staking_asset: Asset,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    match staking_asset {
        Asset {
            amount,
            info: AssetInfo::NativeToken { denom },
        } => daodao_native_staking_msg(
            dao_name,
            staker_address,
            staking_contract_addr,
            Coin { denom, amount },
        ),
        Asset {
            amount,
            info: AssetInfo::Token { contract_addr },
        } => daodao_cw20_staking_msg(
            dao_name,
            staker_address,
            &contract_addr.to_string(),
            staking_contract_addr,
            amount,
        ),
    }
}

/// Stakes native token to daodao dao
pub fn daodao_native_staking_msg<T, U>(
    dao_name: String,
    staker_address: &T,
    staking_contract_addr: &U,
    coin: cosmwasm_std::Coin,
) -> DestinationResult
where
    T: Into<String> + Display,
    U: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            &staking_contract_addr.to_string(),
            &staker_address.to_string(),
            &cw20_stake::msg::ReceiveMsg::Stake {},
            Some(vec![CsdkCoin {
                denom: coin.denom.clone(),
                amount: coin.amount.to_string(),
            }]),
        )?)],
        events: vec![Event::new("dao_stake")
            .add_attribute("dao", dao_name.to_string())
            .add_attribute("amount", coin.to_string())],
    })
}

/// Stakes cw20 token to daodao cw20_stake contract
pub fn daodao_cw20_staking_msg<T, U, V>(
    dao_name: String,
    staker_address: &T,
    cw20_addr: &U,
    staking_contract_addr: &V,
    staking_amount: Uint128,
) -> DestinationResult
where
    T: Into<String> + Display,
    U: Into<String> + Display,
    V: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            cw20_addr.to_string(),
            &staker_address.to_string(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: staking_contract_addr.to_string(),
                amount: staking_amount,
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
            },
            None,
        )?)],
        events: vec![Event::new("dao_stake")
            .add_attribute("dao", dao_name.to_string())
            .add_attribute("amount", staking_amount.to_string())],
    })
}

pub fn send_tokens_msgs<T>(
    sender_addr: &T,
    recipient_addr: &T,
    asset_to_send: Asset,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        msgs: vec![match &asset_to_send.info {
            AssetInfo::NativeToken { denom } => CosmosProtoMsg::Send(MsgSend {
                amount: vec![CsdkCoin {
                    denom: denom.clone(),
                    amount: asset_to_send.amount.into(),
                }],
                from_address: sender_addr.to_string(),
                to_address: recipient_addr.to_string(),
            }),
            AssetInfo::Token { contract_addr } => {
                CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                    contract_addr.clone(),
                    sender_addr,
                    &cw20::Cw20ExecuteMsg::Transfer {
                        recipient: recipient_addr.to_string(),
                        amount: asset_to_send.amount,
                    },
                    None,
                )?)
            }
        }],
        sub_msgs: vec![],
        events: vec![Event::new("send_tokens")
            .add_attribute("to_address", recipient_addr.to_string())
            .add_attribute("amount", asset_to_send.amount.to_string())
            .add_attribute("asset", asset_to_send.info.to_string())],
    })
}
