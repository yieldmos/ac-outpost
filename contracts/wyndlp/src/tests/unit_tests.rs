use std::collections::HashMap;

use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{Addr, Uint128};
use outpost_utils::{
    comp_prefs::{JunoDestinationProject, PoolCatchAllDestinationAction},
    helpers::WyndAssetLPMessages,
    msgs::{create_exec_contract_msg, CosmosProtoMsg},
};
use wyndex::asset::{Asset, AssetInfo, AssetInfoValidated, AssetValidated};

use crate::helpers::{calculate_compound_amounts, fold_wynd_swap_msgs, wynd_join_pool_msgs};

#[test]
fn calc_lp_compound_amounts() {
    assert_eq!(
        calculate_compound_amounts(
            vec![PoolCatchAllDestinationAction {
                destination:
                    outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                        JunoDestinationProject::JunoStaking {
                            validator_address: "btcvaloper1".to_string(),
                        }
                    ),
                amount: 1_000_000_000_000_000_000u128.into()
            }],
            vec![AssetValidated {
                info: AssetInfoValidated::Native("ubtc".to_string()),
                amount: 100u128.into()
            }]
        )
        .unwrap(),
        vec![vec![AssetValidated {
            info: AssetInfoValidated::Native("ubtc".to_string()),
            amount: 100u128.into()
        }],]
    );

    assert_eq!(
        calculate_compound_amounts(
            vec![PoolCatchAllDestinationAction {
                destination:
                    outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                        JunoDestinationProject::JunoStaking {
                            validator_address: "btcvaloper1".to_string(),
                        }
                    ),
                amount: 1_000_000_000_000_000_000u128.into()
            }],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 100u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 1000u128.into()
                }
            ]
        )
        .unwrap(),
        vec![vec![
            AssetValidated {
                info: AssetInfoValidated::Native("ubtc".to_string()),
                amount: 100u128.into()
            },
            AssetValidated {
                info: AssetInfoValidated::Native("ueth".to_string()),
                amount: 1000u128.into()
            },
        ],]
    );

    assert_eq!(
        calculate_compound_amounts(
            vec![
                PoolCatchAllDestinationAction {
                    destination:
                        outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                            JunoDestinationProject::JunoStaking {
                                validator_address: "btcvaloper1".to_string(),
                            }
                        ),
                    amount: 200_000_000_000_000_000u128.into()
                },
                PoolCatchAllDestinationAction {
                    destination:
                        outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                            JunoDestinationProject::JunoStaking {
                                validator_address: "btcvaloper2".to_string(),
                            }
                        ),
                    amount: 800_000_000_000_000_000u128.into()
                }
            ],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 100u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 1000u128.into()
                }
            ]
        )
        .unwrap(),
        vec![
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 20u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 200u128.into()
                },
            ],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 80u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 800u128.into()
                },
            ],
        ]
    );

    assert_eq!(
        calculate_compound_amounts(
            vec![
                PoolCatchAllDestinationAction {
                    destination:
                        outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                            JunoDestinationProject::JunoStaking {
                                validator_address: "btcvaloper1".to_string(),
                            }
                        ),
                    amount: 333_333_333_333_333_333u128.into()
                },
                PoolCatchAllDestinationAction {
                    destination:
                        outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                            JunoDestinationProject::JunoStaking {
                                validator_address: "btcvaloper2".to_string(),
                            }
                        ),
                    amount: 333_333_333_333_333_333u128.into()
                },
                PoolCatchAllDestinationAction {
                    destination:
                        outpost_utils::comp_prefs::PoolCatchAllDestinationProject::BasicDestination(
                            JunoDestinationProject::JunoStaking {
                                validator_address: "btcvaloper3".to_string(),
                            }
                        ),
                    amount: 333_333_333_333_333_333u128.into()
                }
            ],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 100u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 1000u128.into()
                }
            ]
        )
        .unwrap(),
        vec![
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 33u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 333u128.into()
                },
            ],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 33u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 333u128.into()
                },
            ],
            vec![
                AssetValidated {
                    info: AssetInfoValidated::Native("ubtc".to_string()),
                    amount: 34u128.into()
                },
                AssetValidated {
                    info: AssetInfoValidated::Native("ueth".to_string()),
                    amount: 334u128.into()
                },
            ],
        ]
    );
}

#[test]
fn fold_wynd_swaps() {
    assert_eq!(
        fold_wynd_swap_msgs(vec![WyndAssetLPMessages {
            swap_msgs: vec![],
            target_asset_info: Asset {
                info: AssetInfo::Native("ubtc".to_string()),
                amount: 100u128.into(),
            },
        }]),
        (
            vec![],
            HashMap::from([(
                AssetInfo::Native("ubtc".to_string()),
                Uint128::from(100u128)
            )])
        )
    );

    assert_eq!(
        fold_wynd_swap_msgs(vec![
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 100u128.into(),
                },
            },
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 1000u128.into(),
                },
            }
        ]),
        (
            vec![],
            HashMap::from([(
                AssetInfo::Native("ubtc".to_string()),
                Uint128::from(1100u128)
            )])
        )
    );

    assert_eq!(
        fold_wynd_swap_msgs(vec![
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 100u128.into(),
                },
            },
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 1000u128.into(),
                },
            },
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ueth".to_string()),
                    amount: 1000u128.into(),
                },
            }
        ]),
        (
            vec![],
            HashMap::from([
                (
                    AssetInfo::Native("ubtc".to_string()),
                    Uint128::from(1100u128)
                ),
                (
                    AssetInfo::Native("ueth".to_string()),
                    Uint128::from(1000u128)
                )
            ])
        )
    );

    assert_eq!(
        fold_wynd_swap_msgs(vec![
            WyndAssetLPMessages {
                swap_msgs: vec![
                    CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                        sender: "senderaddr".to_string(),
                        contract: "contractaddr".to_string(),
                        msg: vec![],
                        funds: vec![],
                    }),
                    CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                        sender: "senderaddr".to_string(),
                        contract: "contractaddr2".to_string(),
                        msg: vec![],
                        funds: vec![],
                    })
                ],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 100u128.into(),
                },
            },
            WyndAssetLPMessages {
                swap_msgs: vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                    sender: "senderaddr".to_string(),
                    contract: "contractaddr3".to_string(),
                    msg: vec![],
                    funds: vec![],
                })],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ubtc".to_string()),
                    amount: 1000u128.into(),
                },
            },
            WyndAssetLPMessages {
                swap_msgs: vec![],
                target_asset_info: Asset {
                    info: AssetInfo::Native("ueth".to_string()),
                    amount: 1000u128.into(),
                },
            }
        ]),
        (
            vec![
                CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                    sender: "senderaddr".to_string(),
                    contract: "contractaddr".to_string(),
                    msg: vec![],
                    funds: vec![],
                }),
                CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                    sender: "senderaddr".to_string(),
                    contract: "contractaddr2".to_string(),
                    msg: vec![],
                    funds: vec![],
                }),
                CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
                    sender: "senderaddr".to_string(),
                    contract: "contractaddr3".to_string(),
                    msg: vec![],
                    funds: vec![],
                })
            ],
            HashMap::from([
                (
                    AssetInfo::Native("ubtc".to_string()),
                    Uint128::from(1100u128)
                ),
                (
                    AssetInfo::Native("ueth".to_string()),
                    Uint128::from(1000u128)
                )
            ])
        )
    );
}

#[test]
fn generate_join_pool_messages() {
    let delegator_addr = Addr::unchecked("test1");
    let target_pool = "pool1addr".to_string();

    let mut swap_msgs: Vec<CosmosProtoMsg> = vec![];
    let assets: HashMap<AssetInfo, Uint128> = HashMap::from([]);

    let join_pool_msgs = wynd_join_pool_msgs(
        &1u64,
        delegator_addr.to_string(),
        target_pool.clone(),
        &mut swap_msgs,
        assets,
    )
    .unwrap();

    assert_eq!(
        join_pool_msgs,
        vec![CosmosProtoMsg::ExecuteContract(
            create_exec_contract_msg(
                target_pool.clone(),
                &delegator_addr.to_string(),
                &wyndex::pair::ExecuteMsg::ProvideLiquidity {
                    assets: vec![],
                    slippage_tolerance: None,
                    receiver: None,
                },
                None,
            )
            .unwrap()
        )]
    );

    let mut swap_msgs: Vec<CosmosProtoMsg> = vec![];
    let assets: HashMap<AssetInfo, Uint128> = HashMap::from([
        (
            AssetInfo::Native("ubtc".to_string()),
            Uint128::from(1100u128),
        ),
        (
            AssetInfo::Native("ueth".to_string()),
            Uint128::from(1000u128),
        ),
        (
            AssetInfo::Token("contractcw20addrs".to_string()),
            Uint128::from(500u128),
        ),
    ]);

    let join_pool_msgs = wynd_join_pool_msgs(
        &1u64,
        delegator_addr.to_string(),
        target_pool.clone(),
        &mut swap_msgs,
        assets,
    )
    .unwrap();

    assert_eq!(
        join_pool_msgs,
        vec![
            CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    "contractcw20addrs".to_string(),
                    &delegator_addr.to_string(),
                    &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                        spender: target_pool.to_string(),
                        amount: 500u128.into(),
                        expires: Some(cw20::Expiration::AtHeight(2))
                    },
                    None,
                )
                .unwrap()
            ),
            CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    target_pool.clone(),
                    &delegator_addr.to_string(),
                    &wyndex::pair::ExecuteMsg::ProvideLiquidity {
                        assets: vec![
                            Asset {
                                info: AssetInfo::Token("contractcw20addrs".to_string()),
                                amount: 500u128.into()
                            },
                            Asset {
                                info: AssetInfo::Native("ubtc".to_string()),
                                amount: 1100u128.into()
                            },
                            Asset {
                                info: AssetInfo::Native("ueth".to_string()),
                                amount: 1000u128.into()
                            },
                        ],
                        slippage_tolerance: None,
                        receiver: None,
                    },
                    Some(vec![
                        Coin {
                            denom: "ubtc".to_string(),
                            amount: 1100u128.to_string()
                        },
                        Coin {
                            denom: "ueth".to_string(),
                            amount: 1000u128.to_string()
                        },
                    ]),
                )
                .unwrap()
            )
        ]
    );
}

// #[test]
// fn generate_neta_staking_msg() {
//     let delegator_addr = Addr::unchecked("test1");
//     let sim_response = SimulationResponse {
//         referral_amount: 0u128.into(),
//         return_amount: 100u128.into(),
//         spread_amount: 0u128.into(),
//         commission_amount: 0u128.into(),
//     };

//     let expected_msgs: Vec<CosmosProtoMsg> = vec![
//         CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: WYND_CW20_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&wyndex::pair::ExecuteMsg::Swap {
//                 offer_asset: wyndex::asset::Asset {
//                     info: wyndex::asset::AssetInfo::Token("uwynd".to_string()),
//                     amount: 1000u128.into(),
//                 },
//                 ask_asset_info: Some(AssetInfo::Token(NETA_STAKING_ADDR.to_string())),
//                 max_spread: None,
//                 belief_price: None,
//                 to: None,
//                 referral_address: None,
//                 referral_commission: None,
//             })
//             .expect("failed to encode swap msg")
//             .to_vec(),
//             funds: vec![Coin {
//                 amount: 1000u128.to_string(),
//                 denom: "ujuno".to_string(),
//             }],
//         }),
//         CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: NETA_CW20_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
//                 contract: NETA_STAKING_ADDR.to_string(),
//                 amount: 100u128.into(),
//                 msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})
//                     .expect("failed to encode cw20 send msg"),
//             })
//             .expect("failed to encode cw20 send msg")
//             .to_vec(),
//             funds: vec![],
//         }),
//     ];

//     assert_eq!(
//         neta_staking_msgs(delegator_addr, (sim_response,  )).unwrap(),
//         expected_msgs
//     );
// }

// #[test]
// fn generate_juno_staking_msg() {
//     let delegator_addr = Addr::unchecked("test1");
//     let sim_response = SimulationResponse {
//         referral_amount: 0u128.into(),
//         return_amount: 20u128.into(),
//         spread_amount: 0u128.into(),
//         commission_amount: 0u128.into(),
//     };

//     let expected_msgs: Vec<CosmosProtoMsg> = vec![
//         CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: JUNO_WYND_PAIR_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&wyndex::pair::ExecuteMsg::Swap {
//                 offer_asset: wyndex::asset::Asset {
//                     info: wyndex::asset::AssetInfo::Native("ujuno".to_string()),
//                     amount: 1000u128.into(),
//                 },
//                 ask_asset_info: Some(AssetInfo::Token(WYND_CW20_ADDR.to_string())),
//                 max_spread: None,
//                 belief_price: None,
//                 to: None,
//                 referral_address: None,
//                 referral_commission: None,
//             })
//             .expect("failed to encode swap msg")
//             .to_vec(),
//             funds: vec![Coin {
//                 amount: 1000u128.to_string(),
//                 denom: "ujuno".to_string(),
//             }],
//         }),
//         CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: WYND_CW20_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&cw20_vesting::ExecuteMsg::Delegate {
//                 amount: 2000u128.into(),
//                 msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
//                     unbonding_period: 15552000u64,
//                 })
//                 .unwrap(),
//             })
//             .expect("failed to encode cw20 send msg")
//             .to_vec(),
//             funds: vec![],
//         }),
//     ];

//     assert_eq!(
//         juno_staking_msgs(
//             delegator_addr,
//             1000u128.into(),
//             "ujuno".to_string(),
//             sim_response
//         )
//         .unwrap(),
//         expected_msgs
//     );
// }

// #[test]
// fn generate_wynd_swap_msgs() {
//     let delegator_addr = Addr::unchecked("test1");

//     let expected_msgs: Vec<CosmosProtoMsg> = vec![];

//     assert_eq!(
//         wynd_token_swap(
//             delegator_addr.clone(),
//             100u128.into(),
//             "ujuno".to_string(),
//             AssetInfo::Native("ujuno".to_string())
//         )
//         .unwrap(),
//         expected_msgs
//     );

//     let expected_msgs: Vec<CosmosProtoMsg> =
//         vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: WYND_MULTI_HOP_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
//                 operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
//                     offer_asset_info: AssetInfo::Native("ujuno".to_string()),
//                     ask_asset_info: AssetInfo::Native("uusdc".to_string()),
//                 }],
//                 receiver: None,
//                 max_spread: None,
//                 minimum_receive: None,
//                 referral_address: None,
//                 referral_commission: None,
//             })
//             .expect("failed to encode swap msg")
//             .to_vec(),
//             funds: vec![Coin {
//                 amount: 100u128.to_string(),
//                 denom: "ujuno".to_string(),
//             }],
//         })];

//     assert_eq!(
//         wynd_token_swap(
//             delegator_addr.clone(),
//             100u128.into(),
//             "ujuno".to_string(),
//             AssetInfo::Native("uusdc".to_string())
//         )
//         .unwrap(),
//         expected_msgs
//     );

//     let expected_msgs: Vec<CosmosProtoMsg> =
//         vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//             contract: WYND_MULTI_HOP_ADDR.to_string(),
//             sender: "test1".to_string(),
//             msg: to_binary(&wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
//                 operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
//                     offer_asset_info: AssetInfo::Native("ujuno".to_string()),
//                     ask_asset_info: AssetInfo::Token("junowynd1234".to_string()),
//                 }],
//                 receiver: None,
//                 max_spread: None,
//                 minimum_receive: None,
//                 referral_address: None,
//                 referral_commission: None,
//             })
//             .expect("failed to encode swap msg")
//             .to_vec(),
//             funds: vec![Coin {
//                 amount: 100u128.to_string(),
//                 denom: "ujuno".to_string(),
//             }],
//         })];

//     assert_eq!(
//         wynd_token_swap(
//             delegator_addr.clone(),
//             100u128.into(),
//             "ujuno".to_string(),
//             AssetInfo::Token("junowynd1234".to_string())
//         )
//         .unwrap(),
//         expected_msgs
//     );
// }
