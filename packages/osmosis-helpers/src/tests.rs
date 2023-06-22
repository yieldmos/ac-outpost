use std::collections::HashMap;

use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{Addr, Uint128};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
// use wyndex::asset::{Asset, AssetInfo};

// use crate::wynd_lp::{fold_wynd_swap_msgs, wynd_join_pool_from_map_msgs, WyndAssetLPMessages};

// #[test]
// fn fold_wynd_swaps() {
//     assert_eq!(
//         fold_wynd_swap_msgs(vec![WyndAssetLPMessages {
//             swap_msgs: vec![],
//             target_asset_info: Asset {
//                 info: AssetInfo::Native("ubtc".to_string()),
//                 amount: 100u128.into(),
//             },
//         }]),
//         (
//             vec![],
//             HashMap::from([(
//                 AssetInfo::Native("ubtc".to_string()),
//                 Uint128::from(100u128)
//             )])
//         )
//     );

//     assert_eq!(
//         fold_wynd_swap_msgs(vec![
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 100u128.into(),
//                 },
//             },
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 1000u128.into(),
//                 },
//             }
//         ]),
//         (
//             vec![],
//             HashMap::from([(
//                 AssetInfo::Native("ubtc".to_string()),
//                 Uint128::from(1100u128)
//             )])
//         )
//     );

//     assert_eq!(
//         fold_wynd_swap_msgs(vec![
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 100u128.into(),
//                 },
//             },
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 1000u128.into(),
//                 },
//             },
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ueth".to_string()),
//                     amount: 1000u128.into(),
//                 },
//             }
//         ]),
//         (
//             vec![],
//             HashMap::from([
//                 (
//                     AssetInfo::Native("ubtc".to_string()),
//                     Uint128::from(1100u128)
//                 ),
//                 (
//                     AssetInfo::Native("ueth".to_string()),
//                     Uint128::from(1000u128)
//                 )
//             ])
//         )
//     );

//     assert_eq!(
//         fold_wynd_swap_msgs(vec![
//             WyndAssetLPMessages {
//                 swap_msgs: vec![
//                     CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                         sender: "senderaddr".to_string(),
//                         contract: "contractaddr".to_string(),
//                         msg: vec![],
//                         funds: vec![],
//                     }),
//                     CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                         sender: "senderaddr".to_string(),
//                         contract: "contractaddr2".to_string(),
//                         msg: vec![],
//                         funds: vec![],
//                     })
//                 ],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 100u128.into(),
//                 },
//             },
//             WyndAssetLPMessages {
//                 swap_msgs: vec![CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                     sender: "senderaddr".to_string(),
//                     contract: "contractaddr3".to_string(),
//                     msg: vec![],
//                     funds: vec![],
//                 })],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ubtc".to_string()),
//                     amount: 1000u128.into(),
//                 },
//             },
//             WyndAssetLPMessages {
//                 swap_msgs: vec![],
//                 target_asset_info: Asset {
//                     info: AssetInfo::Native("ueth".to_string()),
//                     amount: 1000u128.into(),
//                 },
//             }
//         ]),
//         (
//             vec![
//                 CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                     sender: "senderaddr".to_string(),
//                     contract: "contractaddr".to_string(),
//                     msg: vec![],
//                     funds: vec![],
//                 }),
//                 CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                     sender: "senderaddr".to_string(),
//                     contract: "contractaddr2".to_string(),
//                     msg: vec![],
//                     funds: vec![],
//                 }),
//                 CosmosProtoMsg::ExecuteContract(MsgExecuteContract {
//                     sender: "senderaddr".to_string(),
//                     contract: "contractaddr3".to_string(),
//                     msg: vec![],
//                     funds: vec![],
//                 })
//             ],
//             HashMap::from([
//                 (
//                     AssetInfo::Native("ubtc".to_string()),
//                     Uint128::from(1100u128)
//                 ),
//                 (
//                     AssetInfo::Native("ueth".to_string()),
//                     Uint128::from(1000u128)
//                 )
//             ])
//         )
//     );
// }

// #[test]
// fn generate_join_pool_messages() {
//     let delegator_addr = Addr::unchecked("test1");
//     let target_pool = "pool1addr".to_string();

//     let mut swap_msgs: Vec<CosmosProtoMsg> = vec![];
//     let assets: HashMap<AssetInfo, Uint128> = HashMap::from([]);

//     let join_pool_msgs = wynd_join_pool_from_map_msgs(
//         &1u64,
//         delegator_addr.to_string(),
//         target_pool.clone(),
//         &mut swap_msgs,
//         assets,
//     )
//     .unwrap();

//     assert_eq!(
//         join_pool_msgs,
//         vec![CosmosProtoMsg::ExecuteContract(
//             create_exec_contract_msg(
//                 target_pool.clone(),
//                 &delegator_addr.to_string(),
//                 &wyndex::pair::ExecuteMsg::ProvideLiquidity {
//                     assets: vec![],
//                     slippage_tolerance: None,
//                     receiver: None,
//                 },
//                 None,
//             )
//             .unwrap()
//         )]
//     );

//     let mut swap_msgs: Vec<CosmosProtoMsg> = vec![];
//     let assets: HashMap<AssetInfo, Uint128> = HashMap::from([
//         (
//             AssetInfo::Native("ubtc".to_string()),
//             Uint128::from(1100u128),
//         ),
//         (
//             AssetInfo::Native("ueth".to_string()),
//             Uint128::from(1000u128),
//         ),
//         (
//             AssetInfo::Token("contractcw20addrs".to_string()),
//             Uint128::from(500u128),
//         ),
//     ]);

//     let join_pool_msgs = wynd_join_pool_from_map_msgs(
//         &1u64,
//         delegator_addr.to_string(),
//         target_pool.clone(),
//         &mut swap_msgs,
//         assets,
//     )
//     .unwrap();

//     assert_eq!(
//         join_pool_msgs,
//         vec![
//             CosmosProtoMsg::ExecuteContract(
//                 create_exec_contract_msg(
//                     "contractcw20addrs".to_string(),
//                     &delegator_addr.to_string(),
//                     &cw20::Cw20ExecuteMsg::IncreaseAllowance {
//                         spender: target_pool.to_string(),
//                         amount: 500u128.into(),
//                         expires: Some(cw20::Expiration::AtHeight(2))
//                     },
//                     None,
//                 )
//                 .unwrap()
//             ),
//             CosmosProtoMsg::ExecuteContract(
//                 create_exec_contract_msg(
//                     target_pool.clone(),
//                     &delegator_addr.to_string(),
//                     &wyndex::pair::ExecuteMsg::ProvideLiquidity {
//                         assets: vec![
//                             Asset {
//                                 info: AssetInfo::Token("contractcw20addrs".to_string()),
//                                 amount: 500u128.into()
//                             },
//                             Asset {
//                                 info: AssetInfo::Native("ubtc".to_string()),
//                                 amount: 1100u128.into()
//                             },
//                             Asset {
//                                 info: AssetInfo::Native("ueth".to_string()),
//                                 amount: 1000u128.into()
//                             },
//                         ],
//                         slippage_tolerance: None,
//                         receiver: None,
//                     },
//                     Some(vec![
//                         Coin {
//                             denom: "ubtc".to_string(),
//                             amount: 1100u128.to_string()
//                         },
//                         Coin {
//                             denom: "ueth".to_string(),
//                             amount: 1000u128.to_string()
//                         },
//                     ]),
//                 )
//                 .unwrap()
//             )
//         ]
//     );
// }
