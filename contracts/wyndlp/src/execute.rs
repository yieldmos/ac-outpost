use std::{collections::HashMap, iter};

use cosmos_sdk_proto::cosmos::{base::v1beta1::Coin, staking::v1beta1::MsgDelegate};
use cosmwasm_std::{
    to_binary, Addr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdError, Uint128,
};
use outpost_utils::{
    comp_prefs::{
        CompoundPrefs, DestinationAction, JunoDestinationProject, PoolCatchAllDestinationAction,
        PoolCatchAllDestinationProject, PoolCompoundPrefs, WyndLPBondingPeriod,
        WyndStakingBondingPeriod,
    },
    msgs::{
        create_exec_contract_msg, create_exec_msg, create_wyndex_swap_msg,
        create_wyndex_swap_msg_with_simulation, create_wyndex_swaps_with_sims, CosmosProtoMsg,
        SwapSimResponse,
    }, helpers::WyndAssetLPMessages,
};
use wyndex::{
    asset::{Asset, AssetInfo, AssetValidated},
    pair::{PairInfo, SimulationResponse},
};
use wyndex_multi_hop::msg::SwapOperation;

use crate::{
    helpers::{
        assign_comp_prefs_to_pools, calculate_compound_amounts, valid_catch_all_pool_prefs,
        valid_pool_prefs, PoolRewardsWithPrefs, fold_wynd_swap_msgs,
    },
    ContractError,
};

pub const WYNDDEX_FACTORY_ADDR: &str =
    "juno16adshp473hd9sruwztdqrtsfckgtd69glqm6sqk0hc4q40c296qsxl3u3s";
pub const WYND_CW20_ADDR: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
pub const WYND_MULTI_HOP_ADDR: &str =
    "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w";
pub const JUNO_WYND_PAIR_ADDR: &str =
    "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf";
pub const NETA_CW20_ADDR: &str = "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr";
pub const NETA_STAKING_ADDR: &str =
    "juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh";

pub fn compound(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    delegator_address: String,
    pool_prefs: Vec<PoolCompoundPrefs>,
    other_pools_prefs: Option<Vec<PoolCatchAllDestinationAction>>,
    current_user_pools: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // validate the pool prefs
    let _ = valid_pool_prefs(pool_prefs.clone())?;

    // if there is a set of catch all pool comp prefs we need to validate that the prefs are valid
    if let Some(other_pool_prefs) = &other_pools_prefs {
        let _ = valid_catch_all_pool_prefs(&other_pool_prefs)?;
    }

    let delegator = deps.api.addr_validate(&delegator_address)?;

    // let pending_staking_rewards = queries::query_pending_wynd_pool_rewards(&deps.querier, &delegator)?;
    let pending_rewards: Vec<(PairInfo, Vec<AssetValidated>)> = vec![];

    let pool_rewards_with_prefs =
        assign_comp_prefs_to_pools(pending_rewards, pool_prefs, &other_pools_prefs);

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let sub_msgs = pool_rewards_with_prefs
        .into_iter()
        .map(|rewards_with_prefs| prefs_to_msgs(&deps.querier, &delegator, rewards_with_prefs))
        .collect::<Result<Vec<Vec<_>>, ContractError>>()?
        .into_iter()
        .flatten()
        .collect();

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, sub_msgs)?;

    Ok(Response::default().add_message(exec_msg))
}

/// Converts the user's compound preferences into a list of CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    querier: &QuerierWrapper,
    target_address: &Addr,
    PoolRewardsWithPrefs {
        pool,
        rewards,
        prefs,
    }: PoolRewardsWithPrefs,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // Generate msg for withdrawing the wynd rewards.
    // This should be the first msgs in the tx so the user has funds to compound
    let mut all_msgs: Vec<CosmosProtoMsg> =
        vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            pool.staking_addr.to_string(),
            &target_address.to_string(),
            &wyndex_stake::msg::ExecuteMsg::WithdrawRewards {
                owner: None,
                receiver: None,
            },
            None,
        )?)];

    // calculates the amount of each token that will be used for compounding each specified "destination project"
    let compound_token_amounts =
        iter::zip(calculate_compound_amounts(prefs.clone(), rewards)?, prefs);

    let mut compounding_msgs: Vec<CosmosProtoMsg> = compound_token_amounts
        .map(
            |(comp_token_amounts, 
                PoolCatchAllDestinationAction { destination, .. })| -> 
                Result<Vec<CosmosProtoMsg>, ContractError> {
                match destination {

                    PoolCatchAllDestinationProject::ReturnToPool => todo!("return to pool"),
                    
                    PoolCatchAllDestinationProject::BasicDestination(JunoDestinationProject::WyndLP {
                            contract_address,
                            bonding_period,
                        }) => {        
                            let pool_info: wyndex::pair::PairInfo = if pool.contract_addr.to_string().eq(&contract_address) {
                                pool.clone()
                            }else {   
                                 querier.query_wasm_smart(
                                contract_address.to_string(),
                                &wyndex::pair::QueryMsg::Pair {},
                            )?};
        
                            join_wynd_pool_msgs(
                                &querier,
                                target_address.clone(),
                                comp_token_amounts,
                                contract_address,
                                bonding_period,
                                pool_info.clone(),
                                querier.query_wasm_smart(
                                    pool_info.liquidity_token,
                                    &cw20::Cw20QueryMsg::Balance {
                                        address: target_address.to_string(),
                                    },
                                )?
                            )
                        },
                    PoolCatchAllDestinationProject::BasicDestination(JunoDestinationProject::JunoStaking { validator_address }) =>
                        juno_staking_msgs(
                            querier, 
                            target_address.clone(),
                            comp_token_amounts,                         
                            validator_address,
                        ),
                    PoolCatchAllDestinationProject::BasicDestination(JunoDestinationProject::NetaStaking {}) => neta_staking_msgs(
                        querier, target_address.clone(),
                        comp_token_amounts
                    ),
                    PoolCatchAllDestinationProject::BasicDestination(JunoDestinationProject::WyndStaking { bonding_period }) =>
                        wynd_staking_msgs(
                            querier, target_address.clone(),
                            comp_token_amounts, bonding_period
                        ),
                    PoolCatchAllDestinationProject::BasicDestination(JunoDestinationProject::TokenSwap { target_denom }) => 
                        token_swap_msgs(
                            target_address.clone(),
                            comp_token_amounts,                    
                            target_denom,
                        ),
               
            } },
        )
        .collect::<Result<Vec<_>, ContractError>>()
        .map(|msgs_list| msgs_list.into_iter().flatten().collect())?;

    all_msgs.append(&mut compounding_msgs);

    Ok(all_msgs)
}

pub fn token_swap_msgs(
    target_address: Addr,
    comp_token_amounts: Vec<AssetValidated>,
    target_denom: AssetInfo,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let swap_msgs = comp_token_amounts
        .iter()
        .map(
            |AssetValidated {
                 info: offer_asset,
                 amount: offer_amount,
             }| {
                create_wyndex_swap_msg(
                    &target_address,
                    *offer_amount,
                    offer_asset.clone().into(),
                    target_denom.clone(),
                    WYND_MULTI_HOP_ADDR.to_string(),
                )
            },
        )
        .collect::<Result<Vec<_>, StdError>>()?;

    Ok(swap_msgs.into_iter().flatten().collect())
}

pub fn wynd_staking_msgs(
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amounts: Vec<AssetValidated>,
    bonding_period: WyndStakingBondingPeriod,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap all pool rewards for wynd
    let SwapSimResponse {
        mut swap_msgs,
        simulated_return_amount: expected_wynd,
        ..
    } = create_wyndex_swaps_with_sims(
        querier,
        &target_address,
        comp_token_amounts.into(),
        AssetInfo::Token(WYND_CW20_ADDR.to_string()),
        WYND_MULTI_HOP_ADDR.to_string(),
    )?;

    // delegate wynd to the staking contract
    let wynd_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        WYND_CW20_ADDR.to_string(),
        &target_address,
        &cw20_vesting::ExecuteMsg::Delegate {
            amount: expected_wynd,
            msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                unbonding_period: bonding_period.into(),
            })?,
        },
        None,
    )?);

    swap_msgs.push(wynd_stake_msg);

    Ok(swap_msgs)
}

pub fn neta_staking_msgs(
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amounts: Vec<AssetValidated>,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap all pool rewards for neta
    let SwapSimResponse {
        mut swap_msgs,
        simulated_return_amount: expected_neta,
        ..
    } = create_wyndex_swaps_with_sims(
        querier,
        &target_address,
        comp_token_amounts.into(),
        AssetInfo::Token(NETA_CW20_ADDR.to_string()),
        WYND_MULTI_HOP_ADDR.to_string(),
    )?;

    // stake neta
    let neta_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        NETA_CW20_ADDR.to_string(),
        &target_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: NETA_STAKING_ADDR.to_string(),
            amount: expected_neta,
            msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
        },
        None,
    )?);

    swap_msgs.push(neta_stake_msg);

    Ok(swap_msgs)
}

pub fn juno_staking_msgs(
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amounts: Vec<AssetValidated>,
    validator_address: String,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // generate the swap messages on whatever pool rewards there were
    let SwapSimResponse {
        mut swap_msgs,
        simulated_return_amount: expected_juno,
        ..
    } = create_wyndex_swaps_with_sims(
        querier,
        &target_address,
        comp_token_amounts.into(),
        AssetInfo::Native("ujuno".to_string()),
        WYND_MULTI_HOP_ADDR.to_string(),
    )?;

    // stake juno
    let juno_stake_msg = CosmosProtoMsg::Delegate(MsgDelegate {
        validator_address,
        amount: Some(Coin {
            denom: "ujuno".to_string(),
            amount: expected_juno.into(),
        }),
        delegator_address: target_address.to_string(),
    });

    swap_msgs.push(juno_stake_msg);

    Ok(swap_msgs)
}

pub fn join_wynd_pool_msgs(
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amounts: Vec<AssetValidated>,
    pool_contract_address: String,
    bonding_period: WyndLPBondingPeriod,
    pool_info: wyndex::pair::PairInfo,
    existing_lp_tokens: cw20::BalanceResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let swap_msgs: Vec<WyndAssetLPMessages> = match &comp_token_amounts[..] {
        [AssetValidated {
            info: reward_asset,
            amount,
        }] => {
            let (first_swap_msgs, first_swap_estimate) = create_wyndex_swap_msg_with_simulation(
                querier,
                &target_address,
                *amount / Uint128::from(2u128),
                reward_asset.clone().into(),
                pool_info.asset_infos[0].clone().into(),
                WYND_MULTI_HOP_ADDR.to_string(),
            )?;

            let (second_swap_msgs, second_swap_estimate) = create_wyndex_swap_msg_with_simulation(
                querier,
                &target_address,
                *amount / Uint128::from(2u128),
                reward_asset.clone().into(),
                pool_info.asset_infos[1].clone().into(),
                WYND_MULTI_HOP_ADDR.to_string(),
            )?;

            Ok(vec![
                WyndAssetLPMessages {
                    swap_msgs: first_swap_msgs,
                    target_asset_info: Asset {
                        info: pool_info.asset_infos[0].clone().into(),
                        amount: first_swap_estimate,
                    },
                },
                WyndAssetLPMessages {
                    swap_msgs: second_swap_msgs,
                    target_asset_info: Asset {
                        info: pool_info.asset_infos[1].clone().into(),
                        amount: second_swap_estimate,
                    },
                },
            ])
        }
        _ => Err(ContractError::NotImplemented {}),
    }?;

    let (mut swap_msgs, assets)= fold_wynd_swap_msgs(swap_msgs);

    unimplemented!();

    

    // if !existing_lp_tokens.balance.is_zero() {
    //     swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
    //         pool_info.liquidity_token.to_string(),
    //         &target_address,
    //         &cw20::Cw20ExecuteMsg::Send {
    //             contract: pool_info.staking_addr.to_string(),
    //             amount: existing_lp_tokens.balance,
    //             msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
    //                 unbonding_period: bonding_period.into(),
    //             })?,
    //         },
    //         None,
    //     )?));
    // }

    // let asset_count: u128 = pool_info.asset_infos.len().try_into().unwrap();
    // let wynd_amount_per_asset: Uint128 =
    //     comp_token_amount.checked_div_floor((asset_count, 1u128))?;

    // let pool_assets = wynd_lp_asset_swaps(
    //     querier,
    //     &staking_denom,
    //     &pool_contract_address,
    //     &wynd_amount_per_asset,
    //     &pool_info,
    //     &target_address,
    // )?;

    // let pool_join_funds: Vec<Asset> = pool_assets
    //     .iter()
    //     .map(
    //         |WyndAssetLPMessages {
    //              target_asset_info, ..
    //          }| target_asset_info.clone(),
    //     )
    //     .collect::<Vec<_>>();
    // let native_funds: Vec<Coin> = pool_assets
    //     .iter()
    //     .filter_map(
    //         |WyndAssetLPMessages {
    //              target_asset_info, ..
    //          }| {
    //             if let Asset {
    //                 info: AssetInfo::Native(native_denom),
    //                 amount,
    //             } = target_asset_info
    //             {
    //                 Some(Coin {
    //                     denom: native_denom.clone(),
    //                     amount: amount.to_string(),
    //                 })
    //             } else {
    //                 None
    //             }
    //         },
    //     )
    //     .collect::<Vec<_>>();

    // let mut swap_msgs: Vec<CosmosProtoMsg> = pool_assets
    //     .iter()
    //     .flat_map(|WyndAssetLPMessages { swap_msgs, .. }| swap_msgs.clone())
    //     .collect::<Vec<_>>();

    // swap_msgs.append(&mut vec![
    //     CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
    //         pool_contract_address,
    //         &target_address,
    //         &wyndex::pair::ExecuteMsg::ProvideLiquidity {
    //             assets: pool_join_funds,
    //             slippage_tolerance: None,
    //             receiver: None,
    //         },
    //         Some(native_funds),
    //     )?),
    //     // CosmosProtoMsg::ExecuteContract(
    //     //     create_exec_contract_msg(
    //     //         &pool_info.staking_addr.to_string(),
    //     //         &target_address,
    //     //         &cw20::Cw20ExecuteMsg::Send {
    //     //             contract: pool_info.staking_addr.to_string(),
    //     //             amount: todo!("set estimated lp tokens"),
    //     //             msg: to_binary(
    //     //                 &wyndex_stake::msg::ReceiveDelegationMsg::Delegate {
    //     //                     unbonding_period: bonding_period.into(),

    //     //             } )? ,
    //     //         },
    //     //         None
    //     //     )?)
    // ]);

    // if !existing_lp_tokens.balance.is_zero() {
    //     swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
    //         pool_info.liquidity_token.to_string(),
    //         &target_address,
    //         &cw20::Cw20ExecuteMsg::Send {
    //             contract: pool_info.staking_addr.to_string(),
    //             amount: existing_lp_tokens.balance,
    //             msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
    //                 unbonding_period: bonding_period.into(),
    //             })?,
    //         },
    //         None,
    //     )?));
    // }

    // Ok(swap_msgs)
    // // will need to update things to utilize the routes from the factory
    // // wyndex::factory::ROUTE;
}



/// Generates the wyndex swap messages and IncreaseAllowance (for cw20) messages that are needed before the actual pool can be entered.
/// These messages should ensure that we have the correct amount of assets in the pool contract
pub fn wynd_lp_asset_swaps(
    querier: &QuerierWrapper,
    staking_denom: &String,
    _pool_contract_address: &str,
    wynd_amount_per_asset: &Uint128,
    pool_info: &PairInfo,
    target_address: &Addr,
) -> Result<Vec<WyndAssetLPMessages>, ContractError> {
    pool_info
        .asset_infos
        .iter()
        // map over each asset in the pool to generate the swap msgs and the target asset info
        .map(|asset| -> Result<WyndAssetLPMessages, ContractError> {
            let (swap_msgs, target_token_amount) = create_wyndex_swap_msg_with_simulation(
                querier,
                target_address,
                *wynd_amount_per_asset,
                AssetInfo::Token(staking_denom.clone()),
                asset.clone().into(),
                WYND_MULTI_HOP_ADDR.to_string(),
            )?;

            Ok(WyndAssetLPMessages {
                swap_msgs,
                target_asset_info: Asset {
                    info: asset.clone().into(),
                    amount: target_token_amount,
                },
            })
        })
        .collect()
}
