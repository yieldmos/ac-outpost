use std::iter;

use cosmos_sdk_proto::{
    cosmos::{
        base::v1beta1::Coin, distribution::v1beta1::MsgWithdrawDelegatorReward,
        staking::v1beta1::MsgDelegate,
    },
    Any,
};
use cosmwasm_std::{
    to_binary, Addr, Decimal, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128,
};
use wyndex::{
    asset::{Asset, AssetInfo, AssetInfoValidated},
    pair::{PairInfo, SimulationResponse},
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    generate_exec::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
    helpers::{calculate_compound_amounts, prefs_sum_to_one},
    msg::{
        CompoundPrefs, DestinationAction, DestinationProject, RelativeQty, WyndLPBondingPeriod,
        WyndStakingBondingPeriod,
    },
    queries::{self, query_juno_neta_swap, query_juno_wynd_swap},
    ContractError,
};

const WYND_CW20_ADDR: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
const _WYJUNO_CW20_ADDR: &str = "juno1snv8z7j75jwfce4uhkjh5fedpxjnrx9v20ffflzws57atshr79yqnw032r";
const _WYND_STAKING_ADDR: &str = "juno1sy9mlw47w44f94zea7g98y5ff4cvtc8rfv75jgwphlet83wlf4ssa050mv";
pub const WYND_MULTI_HOP_ADDR: &str =
    "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w";
pub const JUNO_NETA_PAIR_ADDR: &str =
    "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra";
pub const JUNO_WYND_PAIR_ADDR: &str =
    "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf";
// const JUNO_WY_JUNO_PAIR_ADDR: &str =
//     "juno1f9c60hyvzys5h7q0y4e995n8r9cchgpy8p3k4kw3sqsmut95ankq0chfv0";
const NETA_CW20_ADDR: &str = "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr";
const NETA_STAKING_ADDR: &str = "juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh";

pub fn compound(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    delegator_address: String,
    comp_prefs: CompoundPrefs,
) -> Result<Response, ContractError> {
    if !prefs_sum_to_one(&comp_prefs)? {
        return Err(ContractError::InvalidPrefQtys);
    }
    let delegator = deps.api.addr_validate(&delegator_address)?;

    let pending_rewards = queries::query_pending_rewards(&deps.querier, &delegator)?;

    let sub_msgs = prefs_to_msgs(
        deps.querier.query_bonded_denom()?,
        &delegator,
        &pending_rewards,
        comp_prefs,
        deps.querier,
    )?;
    let any_msgs: Vec<Any> = sub_msgs
        .iter()
        .map(|msg| -> Result<Any, ContractError> { msg.try_into() })
        .fold(Ok(vec![]), |acc, msg| -> Result<Vec<Any>, ContractError> {
            match (acc, msg) {
                (Ok(mut acc), Ok(msg)) => {
                    acc.push(msg);
                    Ok(acc)
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        })?;

    let exec_msg = create_exec_msg(&env.contract.address, &any_msgs);

    Ok(Response::default().add_message(exec_msg))
}

pub fn prefs_to_msgs(
    staking_denom: String,
    target_address: &Addr,
    AllPendingRewards {
        rewards: pending_rewards,
        total: total_rewards,
    }: &AllPendingRewards,
    comp_prefs: CompoundPrefs,
    querier: QuerierWrapper,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let mut withdraw_rewards_msgs: Vec<CosmosProtoMsg> = pending_rewards
        .iter()
        .map(|PendingReward { validator, .. }| {
            CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
                validator_address: validator.to_string(),
                delegator_address: target_address.to_string(),
            })
        })
        .collect();

    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative.clone(),
    );

    let compounding_msgs: Result<Vec<CosmosProtoMsg>, ContractError> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<Vec<CosmosProtoMsg>, ContractError> { match destination {
                DestinationProject::JunoStaking { validator_address } => {
                    Ok(vec![CosmosProtoMsg::Delegate(MsgDelegate {
                        validator_address,
                        amount: Some(Coin {
                            denom: total_rewards.denom.clone(),
                            amount: comp_token_amount.into(),
                        }),
                        delegator_address: target_address.to_string(),
                    })])
                },
                DestinationProject::NetaStaking {} => neta_staking_msgs(                    
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    query_juno_neta_swap(&querier,comp_token_amount)?
                    
                ),
                DestinationProject::WyndStaking { bonding_period } =>
                 wynd_staking_msgs(
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    bonding_period,
                    query_juno_wynd_swap(&querier, comp_token_amount)?
                    
                ),
                DestinationProject::TokenSwap { target_denom } => wynd_token_swap(
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    target_denom,                    
                ),
                DestinationProject::WyndLP {
                    contract_address,
                    bonding_period,
                } => {

                    let pool_info: wyndex::pair::PairInfo = querier.query_wasm_smart(
                        contract_address.to_string(),
                        &wyndex::pair::QueryMsg::Pair {},
                    )?;

                    join_wynd_pool_msgs(
                        &querier,
                        target_address.clone(),
                        comp_token_amount,
                        staking_denom.clone(),
                        contract_address,
                        bonding_period,
                         pool_info.clone(),
                         querier.query_wasm_smart(
                            pool_info.liquidity_token.clone(),
                            &cw20::Cw20QueryMsg::Balance {
                                address: target_address.to_string(),
                            },
                        )?
                    )},
            } },
        )
        .fold(Ok(vec![]), |acc, msgs|
        -> Result<Vec<CosmosProtoMsg>, ContractError> {
            match (acc, msgs) {
                (Ok(acc), Ok(msgs)) => {
                    Ok([acc.as_slice(), msgs.as_slice()].concat())
                }
                (Err (e), _) | (_, Err (e)) => Err(e),
            }
        });

    withdraw_rewards_msgs.append(&mut compounding_msgs?);

    Ok(withdraw_rewards_msgs)
}

fn wynd_token_swap(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    target_denom: AssetInfo,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    match target_denom {
        // swapping from ujuno to ujuno so nothing to do here
        AssetInfo::Native(target_native_denom) if staking_denom == target_native_denom => {
            Ok(vec![])
        }
        // create the swap via the multihop contract
        _ => Ok(vec![CosmosProtoMsg::ExecuteContract(
            create_exec_contract_msg(
                &WYND_MULTI_HOP_ADDR.to_string(),
                &target_address,
                &wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                    operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                        offer_asset_info: AssetInfo::Native(staking_denom.clone()),
                        ask_asset_info: target_denom,
                    }],
                    receiver: None,
                    max_spread: None,
                    minimum_receive: None,
                    referral_address: None,
                    referral_commission: None,
                },
                Some(vec![Coin {
                    denom: staking_denom,
                    amount: comp_token_amount.to_string(),
                }]),
            )?,
        )]),
    }
}

fn neta_staking_msgs(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    SimulationResponse {
        return_amount: expected_neta,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let neta_swap_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        &JUNO_NETA_PAIR_ADDR.to_string(),
        &target_address,
        &wyndex::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::Native(staking_denom.clone()),
                amount: comp_token_amount,
            },
            ask_asset_info: Some(AssetInfo::Token(NETA_STAKING_ADDR.to_string())),
            max_spread: None,
            belief_price: None,
            to: None,
            referral_address: None,
            referral_commission: None,
        },
        Some(vec![Coin {
            denom: staking_denom,
            amount: comp_token_amount.to_string(),
        }]),
    )?);

    let neta_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        &NETA_CW20_ADDR.to_string(),
        &target_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: NETA_STAKING_ADDR.to_string(),
            amount: expected_neta,
            msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
        },
        None,
    )?);

    Ok(vec![neta_swap_msg, neta_stake_msg])
}

fn wynd_staking_msgs(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    bonding_period: WyndStakingBondingPeriod,
    SimulationResponse {
        return_amount: expected_wynd,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let wynd_swap_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        &JUNO_WYND_PAIR_ADDR.to_string(),
        &target_address,
        &wyndex::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::Native(staking_denom.clone()),
                amount: comp_token_amount,
            },
            ask_asset_info: Some(AssetInfo::Token(WYND_CW20_ADDR.to_string())),
            max_spread: None,
            belief_price: None,
            to: None,
            referral_address: None,
            referral_commission: None,
        },
        Some(vec![Coin {
            denom: staking_denom,
            amount: comp_token_amount.to_string(),
        }]),
    )?);

    let wynd_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        &WYND_CW20_ADDR.to_string(),
        &target_address,
        &cw20_vesting::ExecuteMsg::Delegate {
            amount: expected_wynd,
            msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                unbonding_period: bonding_period.into(),
            })?,
        },
        None,
    )?);

    Ok(vec![wynd_swap_msg, wynd_stake_msg])
}

fn join_wynd_pool_msgs(
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    pool_contract_address: String,
    bonding_period: WyndLPBondingPeriod,
    pool_info: wyndex::pair::PairInfo,
    existing_lp_tokens: cw20::BalanceResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // let pool_info: wyndex::pair::PoolResponse = querier.query_wasm_smart(
    //     pool_contract_address.to_string(),
    //     &wyndex::pair::QueryMsg::Pool {},
    // )?;

    let asset_count: u128 = pool_info.asset_infos.len().try_into().unwrap();
    let juno_amount_per_asset: Uint128 =
        comp_token_amount.checked_div_floor((asset_count, 1u128))?;

    let pool_assets = wynd_lp_asset_swaps(
        querier,
        &staking_denom,
        &pool_contract_address,
        &juno_amount_per_asset,
        &pool_info,
        &target_address,
    )?;

    let pool_join_funds: Vec<Asset> = pool_assets
        .iter()
        .map(
            |WyndAssetLPMessages {
                 target_asset_info, ..
             }| target_asset_info.clone(),
        )
        .collect::<Vec<_>>();
    let native_funds: Vec<Coin> = pool_assets
        .iter()
        .filter_map(
            |WyndAssetLPMessages {
                 target_asset_info, ..
             }| {
                if let Asset {
                    info: AssetInfo::Native(native_denom),
                    amount,
                } = target_asset_info
                {
                    Some(Coin {
                        denom: native_denom.clone(),
                        amount: amount.to_string(),
                    })
                } else {
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    let mut swap_msgs: Vec<CosmosProtoMsg> = pool_assets
        .iter()
        .flat_map(|WyndAssetLPMessages { swap_msgs, .. }| swap_msgs.clone())
        .collect::<Vec<_>>();

    swap_msgs.append(&mut vec![
        CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            &pool_contract_address,
            &target_address,
            &wyndex::pair::ExecuteMsg::ProvideLiquidity {
                assets: pool_join_funds,
                slippage_tolerance: None,
                receiver: None,
            },
            Some(native_funds),
        )?),
        // CosmosProtoMsg::ExecuteContract(
        //     create_exec_contract_msg(
        //         &pool_info.staking_addr.to_string(),
        //         &target_address,
        //         &cw20::Cw20ExecuteMsg::Send {
        //             contract: pool_info.staking_addr.to_string(),
        //             amount: todo!("set estimated lp tokens"),
        //             msg: to_binary(
        //                 &wyndex_stake::msg::ReceiveDelegationMsg::Delegate {
        //                     unbonding_period: bonding_period.into(),

        //             } )? ,
        //         },
        //         None
        //     )?)
    ]);

    if !existing_lp_tokens.balance.is_zero() {
        swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            &pool_info.liquidity_token.to_string(),
            &target_address,
            &cw20::Cw20ExecuteMsg::Send {
                contract: pool_info.staking_addr.to_string(),
                amount: existing_lp_tokens.balance,
                msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                    unbonding_period: bonding_period.into(),
                })?,
            },
            None,
        )?));
    }

    Ok(swap_msgs)
    // will need to update things to utilize the routes from the factory
    // wyndex::factory::ROUTE;
}

struct WyndAssetLPMessages {
    swap_msgs: Vec<CosmosProtoMsg>,
    target_asset_info: Asset,
}

fn wynd_lp_asset_swaps(
    querier: &QuerierWrapper,
    staking_denom: &String,
    pool_contract_address: &str,
    juno_amount_per_asset: &Uint128,
    pool_info: &PairInfo,
    target_address: &Addr,
) -> Result<Vec<WyndAssetLPMessages>, ContractError> {
    pool_info
        .asset_infos
        .iter()
        .map(|asset| -> Result<WyndAssetLPMessages, ContractError> {
            match asset {
                // if the asset is juno then we can just send it to the pool
                AssetInfoValidated::Native(asset_denom) if asset_denom.eq(staking_denom) => {
                    Ok(WyndAssetLPMessages {
                        swap_msgs: vec![],
                        target_asset_info: Asset {
                            info: wyndex::asset::AssetInfo::Native(staking_denom.to_string()),
                            amount: *juno_amount_per_asset,
                        },
                    })
                }
                asset => {
                    // this swap operation and sim are usable regardless of if we are going to a native or cw20
                    let swap_operation = wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                        offer_asset_info: AssetInfo::Native(staking_denom.clone()),
                        ask_asset_info: asset.clone().into(),
                    };

                    let swap_simulate: wyndex_multi_hop::msg::SimulateSwapOperationsResponse =
                        querier
                            .query_wasm_smart(
                                WYND_MULTI_HOP_ADDR.to_string(),
                                &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
                                    offer_amount: *juno_amount_per_asset,
                                    operations: vec![swap_operation.clone()],
                                    referral: false,
                                    referral_commission: None,
                                },
                            )
                            .map_err(|_| ContractError::SwapSimulationError {
                                from: staking_denom.clone(),
                                to: asset.to_string(),
                            })?;

                    match asset {
                        // target asset is a native token
                        AssetInfoValidated::Native(target_token_denom) => {
                            Ok(WyndAssetLPMessages {
                                swap_msgs: vec![
                                    // the swap to get the target native token
                                    CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                        &WYND_MULTI_HOP_ADDR.to_string(),
                                        &target_address,
                                        &wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                                            operations: vec![swap_operation],
                                            minimum_receive: None,
                                            receiver: None,
                                            max_spread: None,
                                            referral_address: None,
                                            referral_commission: None,
                                        },
                                        Some(vec![Coin {
                                            amount: juno_amount_per_asset.to_string(),
                                            denom: staking_denom.clone(),
                                        }]),
                                    )?),
                                ],
                                target_asset_info: Asset {
                                    info: wyndex::asset::AssetInfo::Native(
                                        target_token_denom.to_string(),
                                    ),
                                    amount: swap_simulate.amount,
                                },
                            })
                        }
                        // target asset is some sort of cw20
                        AssetInfoValidated::Token(cw20_token_addr) => {
                            //the some should be the lp swap and the second should be the amount to be prepared to put into the pool

                            Ok(WyndAssetLPMessages {
                                swap_msgs: vec![
                                    // the swap to get the cw20
                                    CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                        &WYND_MULTI_HOP_ADDR.to_string(),
                                        &target_address,
                                        &wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                                            operations: vec![swap_operation],
                                            receiver: None,
                                            max_spread: None,
                                            minimum_receive: None,
                                            referral_address: None,
                                            referral_commission: None,
                                        },
                                        Some(vec![Coin {
                                            amount: juno_amount_per_asset.to_string(),
                                            denom: staking_denom.clone(),
                                        }]),
                                    )?),
                                    // the allocation of cw20 tokens the user is putting into the pool
                                    CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                        &cw20_token_addr.to_string(),
                                        &target_address,
                                        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                                            spender: pool_contract_address.to_string(),
                                            amount: swap_simulate.amount,
                                            expires: None,
                                        },
                                        None,
                                    )?),
                                ],

                                target_asset_info: Asset {
                                    info: AssetInfo::Token(cw20_token_addr.to_string()),
                                    amount: swap_simulate.amount,
                                },
                            })
                        }
                    }
                }
            }
        })
        .collect()
}
