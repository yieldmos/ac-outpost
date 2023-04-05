use std::str::FromStr;

use cosmos_sdk_proto::cosmos::{
    base::v1beta1::Coin, distribution::v1beta1::MsgWithdrawDelegatorReward,
    staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    generate_exec::CosmosProtoMsg,
    helpers::calculate_compound_amounts,
    msg::{CompoundPrefs, DestinationAction, DestinationProject, RelativeQty},
};

#[test]
fn calculate_compound_amounts_even_test() {
    let percentages = vec![
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
    ];

    let total_amount = Uint128::from(100u128);

    let expected_amounts = vec![
        Uint128::from(25u128),
        Uint128::from(25u128),
        Uint128::from(25u128),
        Uint128::from(25u128),
    ];

    let amounts = calculate_compound_amounts(&percentages, &total_amount).unwrap();

    assert_eq!(amounts, expected_amounts);
}

#[test]
fn calculate_compound_amounts_with_remainder_test() {
    let percentages = vec![
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.3333").unwrap(),
        Decimal::from_str("0.3333").unwrap(),
        Decimal::from_str("0.0834").unwrap(),
    ];

    let total_amount = Uint128::from(100u128);

    let expected_amounts = vec![
        Uint128::from(25u128),
        Uint128::from(33u128),
        Uint128::from(33u128),
        Uint128::from(9u128),
    ];

    let amounts = calculate_compound_amounts(&percentages, &total_amount).unwrap();

    assert_eq!(amounts, expected_amounts);
}

#[test]
fn pref_conversion_compounding_to_single_validator() {
    let _target_address = Addr::unchecked("test1");
    // let pending_rewards = AllPendingRewards {
    //     rewards: vec![PendingReward {
    //         validator: "testvaloper_withpendingrewards".to_string(),
    //         amount: Coin::new(100, "ubtc"),
    //     }],
    //     total: Coin::new(100, "ubtc"),
    // };
    let _comp_prefs = CompoundPrefs {
        relative: vec![DestinationAction {
            destination: DestinationProject::JunoStaking {
                validator_address: "testvaloper_delegationtarget".to_string(),
            },
            amount: RelativeQty {
                quantity: 1000000000000000000u128,
            },
        }],
    };

    let _expected_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
            delegator_address: "test1".to_string(),
            validator_address: "testvaloper_withpendingrewards".to_string(),
        }),
        CosmosProtoMsg::Delegate(MsgDelegate {
            delegator_address: "test1".to_string(),
            validator_address: "testvaloper_delegationtarget".to_string(),
            amount: Some(Coin {
                amount: "100".to_string(),
                denom: "ubtc".to_string(),
            }),
        }),
    ];

    // assert_eq!(
    //     prefs_to_msgs(&target_address, &pending_rewards, &comp_prefs).unwrap(),
    //     expected_msgs
    // );
}

#[test]
fn pref_conversion_compounding_to_multiple_validators() {
    let _target_address = Addr::unchecked("test1");

    let _comp_prefs = CompoundPrefs {
        relative: vec![
            DestinationAction {
                destination: DestinationProject::JunoStaking {
                    validator_address: "testvaloper_delegationtarget".to_string(),
                },
                amount: RelativeQty {
                    quantity: 200000000000000000u128,
                },
            },
            DestinationAction {
                destination: DestinationProject::JunoStaking {
                    validator_address: "testvaloper_delegationtarget2".to_string(),
                },
                amount: RelativeQty {
                    quantity: 800000000000000000u128,
                },
            },
        ],
    };

    let _expected_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
            delegator_address: "test1".to_string(),
            validator_address: "testvaloper_withpendingrewards".to_string(),
        }),
        CosmosProtoMsg::Delegate(MsgDelegate {
            delegator_address: "test1".to_string(),
            validator_address: "testvaloper_delegationtarget".to_string(),
            amount: Some(Coin {
                amount: "20".to_string(),
                denom: "ubtc".to_string(),
            }),
        }),
        CosmosProtoMsg::Delegate(MsgDelegate {
            delegator_address: "test1".to_string(),
            validator_address: "testvaloper_delegationtarget2".to_string(),
            amount: Some(Coin {
                amount: "80".to_string(),
                denom: "ubtc".to_string(),
            }),
        }),
    ];

    // assert_eq!(
    //     prefs_to_msgs(&target_address, &pending_rewards, &comp_prefs).unwrap(),
    //     expected_msgs
    // );
}

#[test]
fn pref_conversion_compounding_to_multiple_validators_with_multiple_rewards() {
    let target_address = Addr::unchecked("test1");
    let _pending_rewards = AllPendingRewards {
        rewards: vec![
            PendingReward {
                validator: "testvaloper_withpendingrewards".to_string(),
                amount: cosmwasm_std::Coin::new(100, "ubtc"),
            },
            PendingReward {
                validator: "testvaloper_withpendingrewards2".to_string(),
                amount: cosmwasm_std::Coin::new(200, "ubtc"),
            },
        ],
        total: cosmwasm_std::Coin::new(300, "ubtc"),
    };

    let _comp_prefs = CompoundPrefs {
        relative: vec![
            DestinationAction {
                destination: DestinationProject::JunoStaking {
                    validator_address: "testvaloper_delegationtarget".to_string(),
                },
                amount: RelativeQty {
                    quantity: 200000000000000000u128,
                },
            },
            DestinationAction {
                destination: DestinationProject::JunoStaking {
                    validator_address: "testvaloper_delegationtarget2".to_string(),
                },
                amount: RelativeQty {
                    quantity: 800000000000000000u128,
                },
            },
        ],
    };

    let _expected_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
            validator_address: "testvaloper_withpendingrewards".to_string(),
            delegator_address: target_address.to_string(),
        }),
        CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
            validator_address: "testvaloper_withpendingrewards2".to_string(),
            delegator_address: target_address.to_string(),
        }),
        CosmosProtoMsg::Delegate(MsgDelegate {
            validator_address: "testvaloper_delegationtarget".to_string(),
            delegator_address: target_address.to_string(),
            amount: Some(Coin {
                amount: 60.to_string(),
                denom: "ubtc".to_string(),
            }),
        }),
        CosmosProtoMsg::Delegate(MsgDelegate {
            validator_address: "testvaloper_delegationtarget2".to_string(),
            delegator_address: target_address.to_string(),
            amount: Some(Coin {
                amount: 240.to_string(),
                denom: "ubtc".to_string(),
            }),
        }),
    ];

    // assert_eq!(
    //     prefs_to_msgs(
    //         &target_address,
    //         &pending_rewards,
    //         &comp_prefs,
    //         QuerierWrapper::new(&MockQuerier::new(&[(
    //             &"MOCK_CONTRACT_ADDR".to_string(),
    //             &[cosmwasm_std::Coin::new(0, "ubtc")]
    //         )]))
    //     )
    //     .unwrap(),
    //     expected_msgs
    // );
}
