use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{to_binary, Addr};
use outpost_utils::msg_gen::CosmosProtoMsg;
use wyndex::{asset::AssetInfo, pair::SimulationResponse};

use crate::execute::{neta_staking_msgs, wynd_staking_msgs};

const JUNO_NETA_PAIR_ADDR: &str = "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra";
const NETA_CW20_ADDR: &str = "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr";
const NETA_STAKING_ADDR: &str = "juno1q2qjg8x9q3zj6x5q2qjg8x9q3zj6x5q2qjg8x9";
const WYND_CW20_ADDR: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
const JUNO_WYND_PAIR_ADDR: &str = "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf";

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
                ask_asset_info: Some(AssetInfo::Token(NETA_CW20_ADDR.to_string())),
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
                msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).expect("failed to encode cw20 send msg"),
            })
            .expect("failed to encode cw20 send msg")
            .to_vec(),
            funds: vec![],
        }),
    ];

    assert_eq!(
        neta_staking_msgs(
            NETA_CW20_ADDR,
            JUNO_NETA_PAIR_ADDR,
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
            WYND_CW20_ADDR,
            JUNO_WYND_PAIR_ADDR,
            delegator_addr,
            1000u128.into(),
            "ujuno".to_string(),
            outpost_utils::juno_comp_prefs::WyndStakingBondingPeriod::OneHundredEightyDays,
            sim_response
        )
        .unwrap(),
        expected_msgs
    );
}
