use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{to_binary, Addr};
use outpost_utils::msgs::CosmosProtoMsg;
use wyndex::{asset::AssetInfo, pair::SimulationResponse};

use crate::execute::{
    neta_staking_msgs, wynd_staking_msgs, wynd_token_swap, JUNO_NETA_PAIR_ADDR,
    JUNO_WYND_PAIR_ADDR, NETA_CW20_ADDR, NETA_STAKING_ADDR, WYND_CW20_ADDR, WYND_MULTI_HOP_ADDR,
};

#[test]
fn generate_neta_staking_msg() {
    let delegator_addr = Addr::unchecked("test1");
    let sim_response = SimulationResponse {
        referral_amount: 0u128.into(),
        return_amount: 100u128.into(),
        spread_amount: 0u128.into(),
        commission_amount: 0u128.into(),
    };

    let expected_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: JUNO_NETA_PAIR_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&wyndex::pair::ExecuteMsg::Swap {
                offer_asset: wyndex::asset::Asset {
                    info: wyndex::asset::AssetInfo::Native("ujuno".to_string()),
                    amount: 1000u128.into(),
                },
                ask_asset_info: Some(AssetInfo::Token(NETA_STAKING_ADDR.to_string())),
                max_spread: None,
                belief_price: None,
                to: None,
                referral_address: None,
                referral_commission: None,
            })
            .expect("failed to encode swap msg")
            .to_vec(),
            funds: vec![Coin {
                amount: 1000u128.to_string(),
                denom: "ujuno".to_string(),
            }],
        }),
        CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: NETA_CW20_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                contract: NETA_STAKING_ADDR.to_string(),
                amount: 100u128.into(),
                msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})
                    .expect("failed to encode cw20 send msg"),
            })
            .expect("failed to encode cw20 send msg")
            .to_vec(),
            funds: vec![],
        }),
    ];

    assert_eq!(
        neta_staking_msgs(
            delegator_addr,
            1000u128.into(),
            "ujuno".to_string(),
            sim_response
        )
        .unwrap(),
        expected_msgs
    );
}

#[test]
fn generate_wynd_staking_msg() {
    let delegator_addr = Addr::unchecked("test1");
    let sim_response = SimulationResponse {
        referral_amount: 0u128.into(),
        return_amount: 2000u128.into(),
        spread_amount: 0u128.into(),
        commission_amount: 0u128.into(),
    };

    let expected_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: JUNO_WYND_PAIR_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&wyndex::pair::ExecuteMsg::Swap {
                offer_asset: wyndex::asset::Asset {
                    info: wyndex::asset::AssetInfo::Native("ujuno".to_string()),
                    amount: 1000u128.into(),
                },
                ask_asset_info: Some(AssetInfo::Token(WYND_CW20_ADDR.to_string())),
                max_spread: None,
                belief_price: None,
                to: None,
                referral_address: None,
                referral_commission: None,
            })
            .expect("failed to encode swap msg")
            .to_vec(),
            funds: vec![Coin {
                amount: 1000u128.to_string(),
                denom: "ujuno".to_string(),
            }],
        }),
        CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: WYND_CW20_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&cw20_vesting::ExecuteMsg::Delegate {
                amount: 2000u128.into(),
                msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                    unbonding_period: 15552000u64,
                })
                .unwrap(),
            })
            .expect("failed to encode cw20 send msg")
            .to_vec(),
            funds: vec![],
        }),
    ];

    assert_eq!(
        wynd_staking_msgs(
            delegator_addr,
            1000u128.into(),
            "ujuno".to_string(),
            outpost_utils::comp_prefs::WyndStakingBondingPeriod::OneHundredEightyDays,
            sim_response
        )
        .unwrap(),
        expected_msgs
    );
}

#[test]
fn generate_wynd_swap_msgs() {
    let delegator_addr = Addr::unchecked("test1");

    let expected_msgs: Vec<CosmosProtoMsg> = vec![];

    assert_eq!(
        wynd_token_swap(
            delegator_addr.clone(),
            100u128.into(),
            "ujuno".to_string(),
            AssetInfo::Native("ujuno".to_string())
        )
        .unwrap(),
        expected_msgs
    );

    let expected_msgs: Vec<CosmosProtoMsg> =
        vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: WYND_MULTI_HOP_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                    offer_asset_info: AssetInfo::Native("ujuno".to_string()),
                    ask_asset_info: AssetInfo::Native("uusdc".to_string()),
                }],
                receiver: None,
                max_spread: None,
                minimum_receive: None,
                referral_address: None,
                referral_commission: None,
            })
            .expect("failed to encode swap msg")
            .to_vec(),
            funds: vec![Coin {
                amount: 100u128.to_string(),
                denom: "ujuno".to_string(),
            }],
        })];

    assert_eq!(
        wynd_token_swap(
            delegator_addr.clone(),
            100u128.into(),
            "ujuno".to_string(),
            AssetInfo::Native("uusdc".to_string())
        )
        .unwrap(),
        expected_msgs
    );

    let expected_msgs: Vec<CosmosProtoMsg> =
        vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
            contract: WYND_MULTI_HOP_ADDR.to_string(),
            sender: "test1".to_string(),
            msg: to_binary(&wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                    offer_asset_info: AssetInfo::Native("ujuno".to_string()),
                    ask_asset_info: AssetInfo::Token("junowynd1234".to_string()),
                }],
                receiver: None,
                max_spread: None,
                minimum_receive: None,
                referral_address: None,
                referral_commission: None,
            })
            .expect("failed to encode swap msg")
            .to_vec(),
            funds: vec![Coin {
                amount: 100u128.to_string(),
                denom: "ujuno".to_string(),
            }],
        })];

    assert_eq!(
        wynd_token_swap(
            delegator_addr.clone(),
            100u128.into(),
            "ujuno".to_string(),
            AssetInfo::Token("junowynd1234".to_string())
        )
        .unwrap(),
        expected_msgs
    );
}
