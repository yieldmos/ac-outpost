use std::iter;

use crate::{
    helpers::{
        assign_comp_prefs_to_pools, calculate_compound_amounts, fold_wynd_swap_msgs,
        valid_catch_all_pool_prefs, valid_pool_prefs, wynd_join_pool_msgs, PoolRewardsWithPrefs,
    },
    queries::{
        check_user_pools_for_rewards, get_max_user_pool_bonding_period, query_current_user_pools,
    },
    ContractError,
};
use cosmos_sdk_proto::cosmos::{base::v1beta1::Coin, staking::v1beta1::MsgDelegate};
use cosmwasm_std::{
    to_binary, Addr, BlockInfo, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdError,
    Uint128,
};
use outpost_utils::{
    comp_prefs::{
        JunoDestinationProject, PoolCatchAllDestinationAction, PoolCatchAllDestinationProject,
        PoolCompoundPrefs, WyndLPBondingPeriod, WyndStakingBondingPeriod,
    },
    helpers::WyndAssetLPMessages,
    msgs::{
        create_exec_contract_msg, create_exec_msg, create_wyndex_swap_msg,
        create_wyndex_swap_msg_with_simulation, create_wyndex_swaps_with_sims, CosmosProtoMsg,
        SwapSimResponse,
    },
    queries::simulate_multiple_swaps,
};
use wyndex::{
    asset::{Asset, AssetInfo, AssetValidated},
    pair::{PairInfo, SimulationResponse},
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
    current_user_pools: Option<Vec<PairInfo>>,
) -> Result<Response, ContractError> {
    // validate the pool prefs
    let _ = valid_pool_prefs(pool_prefs.clone())?;

    // if there is a set of catch all pool comp prefs we need to validate that the prefs are valid
    if let Some(other_pool_prefs) = &other_pools_prefs {
        let _ = valid_catch_all_pool_prefs(&other_pool_prefs)?;
    }

    // validate that the delegator address is good
    let delegator = deps.api.addr_validate(&delegator_address)?;

    // get the list of pools that the user has staked in and the rewards they have pending
    let pending_rewards: Vec<(PairInfo, Vec<AssetValidated>)> = current_user_pools.map_or_else(
        || query_current_user_pools(&deps.querier, &delegator),
        |user_pools| check_user_pools_for_rewards(&deps.querier, &delegator, user_pools),
    )?;

    // pair the rewards and pools with the user's comp prefs so we can generate the msgs later
    let pool_rewards_with_prefs =
        assign_comp_prefs_to_pools(pending_rewards, pool_prefs, &other_pools_prefs);

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let sub_msgs = pool_rewards_with_prefs
        .into_iter()
        .map(|rewards_with_prefs| {
            prefs_to_msgs(&env.block, &deps.querier, &delegator, rewards_with_prefs)
        })
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
    current_block: &BlockInfo,
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
                    PoolCatchAllDestinationProject::ReturnToPool =>  join_wynd_pool_msgs(
                        &current_block.height,
                        &querier,
                        target_address.clone(),
                        comp_token_amounts,
                        // since there's no specified pool to target we have to check to see what
                        // the user's current bonding period is
                        get_max_user_pool_bonding_period(
                            &querier,
                            &pool.staking_addr,
                            &target_address)?,
                        pool.clone(),
                        querier.query_wasm_smart(
                            pool.clone().liquidity_token,
                            &cw20::Cw20QueryMsg::Balance {
                                address: target_address.to_string(),
                            },
                        )?
                    ),

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
                                &current_block.height,
                                &querier,
                                target_address.clone(),
                                comp_token_amounts,
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

/// Generate the messages to join a Wynd LP pool
pub fn join_wynd_pool_msgs(
    current_block_height: &u64,
    querier: &QuerierWrapper,
    target_address: Addr,
    all_reward_tokens: Vec<AssetValidated>,
    bonding_period: WyndLPBondingPeriod,
    pool_info: wyndex::pair::PairInfo,
    existing_lp_tokens: cw20::BalanceResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    let swap_msgs: Vec<WyndAssetLPMessages> = if let [AssetValidated {
        info: reward_asset,
        amount,
    }] = &all_reward_tokens[..]
    {
        // if there's only one reward asset, we can use simplified logic to knock it out with less gas and complexity

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

        vec![
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
        ]
    } else {
        // if there's more than one reward asset, we need to do a more complex simulation to figure out the best way to swap them

        swap_rewards_to_pool_assets(
            querier,
            &target_address,
            &mut simulate_multiple_swaps(
                querier,
                all_reward_tokens,
                pool_info.asset_infos.first().unwrap(),
                &WYND_MULTI_HOP_ADDR.to_string(),
            )?
            .clone(),
            &pool_info,
        )?
    };

    // combine all the swap msgs into a simplified format that we can put into use
    let (mut swap_msgs, assets) = fold_wynd_swap_msgs(swap_msgs);

    // get the list of msgs needed to join the pool after doing the swaps
    let mut join_pool_msgs = wynd_join_pool_msgs(
        current_block_height,
        target_address.to_string(),
        pool_info.staking_addr.to_string(),
        &mut swap_msgs,
        assets,
    )?;

    // this is a stopgap for testing purposes
    // since we can't know yet how many gamm tokens to stake/bond
    // we'll just stake the existing LP tokens besides the ones we're adding in today's compounding
    // in the next compounding we'll get the new LP tokens and stake those
    if !existing_lp_tokens.balance.is_zero() {
        join_pool_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            pool_info.liquidity_token.to_string(),
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

    Ok(join_pool_msgs)
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

pub fn swap_rewards_to_pool_assets(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
    rewards: &mut Vec<(AssetValidated, SimulationResponse)>,
    pool_info: &PairInfo,
) -> Result<Vec<WyndAssetLPMessages>, ContractError> {
    let mut lp_assets: Vec<WyndAssetLPMessages> = vec![];

    // this is the total amount of rewards in terms of the first pool asset
    // we will need to have half of this amount swapped into the second pool asset
    let total_rewards_value: Uint128 = rewards
        .iter()
        .map(|(_, SimulationResponse { return_amount, .. })| return_amount)
        .sum();

    // the amount of the first pool asset that we need to ascertain for both sides of the lp entry
    let (mut asset_one_amount, mut asset_two_amount) = (
        total_rewards_value / Uint128::from(2u128),
        total_rewards_value / Uint128::from(2u128),
    );
    let first_asset = &pool_info.asset_infos[0];
    let second_asset = &pool_info.asset_infos[1];

    // if we have a reward that is the same asset as the first pool asset, we can short cut some logic and save some gas.
    // ignoring this edgecase would mean that we might accidentally swap an asset that can be deposeted as is.
    if let Some((
        found_reward,
        SimulationResponse {
            return_amount: simulated_asset_amount,
            ..
        },
    )) = rewards
        .iter_mut()
        .find(|(reward, _)| reward.info.equal(&first_asset))
    {
        let overlap_amount = asset_one_amount.min(simulated_asset_amount.clone());
        let original_asset_overlap_amount =
            found_reward.amount * simulated_asset_amount.clone() / overlap_amount.clone();

        asset_one_amount -= overlap_amount.clone();
        found_reward.amount -= original_asset_overlap_amount;
        *simulated_asset_amount -= overlap_amount.clone();
        lp_assets.push(WyndAssetLPMessages {
            swap_msgs: vec![],
            target_asset_info: Asset {
                info: found_reward.info.clone().into(),
                amount: original_asset_overlap_amount,
            },
        });
    };

    // do the same shortcut on the other asset if it's available
    if let Some((
        found_reward,
        SimulationResponse {
            return_amount: simulated_asset_amount,
            ..
        },
    )) = rewards
        .iter_mut()
        .find(|(reward, _)| reward.info.equal(&second_asset))
    {
        let overlap_amount = asset_two_amount.min(simulated_asset_amount.clone());
        let original_asset_overlap_amount =
            found_reward.amount * simulated_asset_amount.clone() / overlap_amount.clone();

        asset_two_amount -= overlap_amount.clone();
        found_reward.amount -= original_asset_overlap_amount;
        *simulated_asset_amount -= overlap_amount.clone();
        lp_assets.push(WyndAssetLPMessages {
            swap_msgs: vec![],
            target_asset_info: Asset {
                info: found_reward.info.clone().into(),
                amount: original_asset_overlap_amount,
            },
        });
    };

    // if we exhausted any of the rewards in the previous shortcuts we can remove it from the rewards list
    rewards.retain(|(_, SimulationResponse { return_amount, .. })| !return_amount.is_zero());

    // now we can just cycle through the remaining rewards (which require swaps) and swap them to the pool assets until we've exhaused the amounts needed (specified in the `asset_one_amount`)
    for (
        reward,
        SimulationResponse {
            return_amount: simulated_asset_amount,
            ..
        },
    ) in rewards.iter_mut()
    {
        if asset_one_amount.is_zero() {
            break;
        }

        let overlap_amount = asset_one_amount.min(simulated_asset_amount.clone());
        let original_asset_overlap_amount =
            reward.amount * simulated_asset_amount.clone() / overlap_amount.clone();

        let (swap_msgs, target_token_amount) = create_wyndex_swap_msg_with_simulation(
            querier,
            delegator_addr,
            original_asset_overlap_amount,
            reward.info.clone().into(),
            first_asset.clone().into(),
            WYND_MULTI_HOP_ADDR.to_string(),
        )?;

        asset_one_amount -= overlap_amount.clone();
        reward.amount -= original_asset_overlap_amount;
        *simulated_asset_amount -= overlap_amount.clone();
        lp_assets.push(WyndAssetLPMessages {
            swap_msgs,
            target_asset_info: Asset {
                info: first_asset.clone().into(),
                amount: target_token_amount,
            },
        });
    }

    // if we exhausted any of the rewards in the previous logic
    rewards.retain(|(_, SimulationResponse { return_amount, .. })| !return_amount.is_zero());

    // finally, whatever is remaining should be swapped to the second pool asset
    for (
        reward,
        SimulationResponse {
            return_amount: simulated_asset_amount,
            ..
        },
    ) in rewards.iter_mut()
    {
        let overlap_amount = simulated_asset_amount.clone();
        let original_asset_overlap_amount =
            reward.amount * simulated_asset_amount.clone() / overlap_amount.clone();

        let (swap_msgs, target_token_amount) = create_wyndex_swap_msg_with_simulation(
            querier,
            delegator_addr,
            original_asset_overlap_amount,
            reward.info.clone().into(),
            second_asset.clone().into(),
            WYND_MULTI_HOP_ADDR.to_string(),
        )?;

        lp_assets.push(WyndAssetLPMessages {
            swap_msgs,
            target_asset_info: Asset {
                info: second_asset.clone().into(),
                amount: target_token_amount,
            },
        });
    }

    Ok(lp_assets)

    // TODO: I think there may be another potential optimization here where we look through the reward amounts
    // and see if any grouping of them match exactly with the pool asset needed so that we dont accidentally
    // chop up a reward into multiple swaps when it's not necessary.
    // Until multiple rewards becomes more common this is probably not worth the effort
}
