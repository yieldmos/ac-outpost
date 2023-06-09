use std::iter;

use crate::{
    queries::{self, query_wynd_juno_swap, query_wynd_neta_swap},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};
use cosmos_sdk_proto::cosmos::{base::v1beta1::Coin, staking::v1beta1::MsgDelegate};
use cosmwasm_std::{to_binary, Addr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128};
use outpost_utils::{
    comp_prefs::{CompoundPrefs, DestinationAction, JunoDestinationProject, WyndLPBondingPeriod},
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one},
    msg_gen::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
};
use wynd_helpers::{
    wynd_lp::{wynd_join_pool_msgs, WyndAssetLPMessages},
    wynd_swap::{
        create_wyndex_swap_msg, create_wyndex_swap_msg_with_simulation, wynd_pair_swap_msg,
    },
};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::{PairInfo, SimulationResponse},
};
use wyndex_multi_hop::msg::SwapOperation;

pub const WYND_CW20_ADDR: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
pub const WYND_CW20_STAKING_ADDR: &str =
    "juno1sy9mlw47w44f94zea7g98y5ff4cvtc8rfv75jgwphlet83wlf4ssa050mv";
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
    info: MessageInfo,
    delegator_address: String,
    comp_prefs: CompoundPrefs,
) -> Result<Response, ContractError> {
    // check that the compounding preference quantities are valid
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let delegator = deps.api.addr_validate(&delegator_address)?;

    // validate that the user is authorized to compound
    let _ = is_authorized_compounder(
        deps.as_ref(),
        &info.sender,
        &delegator,
        ADMIN,
        AUTHORIZED_ADDRS,
    )?;

    // get the pending wynd rewards for the user
    let pending_staking_rewards = queries::query_pending_wynd_rewards(&deps.querier, &delegator)?;

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let sub_msgs = prefs_to_msgs(
        &delegator,
        WYND_CW20_ADDR.to_string(),
        pending_staking_rewards,
        comp_prefs,
        deps.querier,
    )?;

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, sub_msgs)?;

    Ok(Response::default().add_message(exec_msg))
}

/// Converts the user's compound preferences into a list of CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    target_address: &Addr,
    staking_denom: String,
    total_rewards: Uint128,
    comp_prefs: CompoundPrefs,
    querier: QuerierWrapper,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // Generate msg for withdrawing the wynd rewards.
    // This should be the first msgs in the tx so the user has funds to compound
    let mut all_msgs: Vec<CosmosProtoMsg> =
        vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            WYND_CW20_STAKING_ADDR.to_string(),
            &target_address.to_string(),
            &wynd_stake::msg::ExecuteMsg::WithdrawRewards {
                owner: None,
                receiver: None,
            },
            None,
        )?)];

    // calculates the amount of ujuno that will be used for each target project accurately
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards)?,
        comp_prefs.relative,
    );

    // the list of all the messages that will be broadcast on behalf of the user based on their comp prefs
    let mut compounding_msgs: Vec<CosmosProtoMsg> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<Vec<CosmosProtoMsg>, ContractError> {
                match destination {
                JunoDestinationProject::JunoStaking { validator_address } =>
                    juno_staking_msgs(target_address.clone(),
                        comp_token_amount,
                         validator_address,
                         query_wynd_juno_swap(&querier, 
                            comp_token_amount)?
                    )
                ,
                JunoDestinationProject::NetaStaking {} => neta_staking_msgs(
                    target_address.clone(),
                    query_wynd_neta_swap(&querier,
                        comp_token_amount)?
                ),
                JunoDestinationProject::WyndStaking { bonding_period } =>
                    // going back to staking wynd is simple here since it requires no swaps
                    // so we can just shoot back a delegate msg and be done
                    Ok(vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                        WYND_CW20_ADDR.to_string(),
                        &target_address,
                        &cw20_vesting::ExecuteMsg::Delegate {
                            amount: comp_token_amount,
                            msg: to_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                                unbonding_period: bonding_period.into(),
                            })?,
                        },
                        None,
                    )?)]),
                JunoDestinationProject::TokenSwap { target_denom } => wynd_token_swap(
                    target_address.clone(),
                    comp_token_amount,
                    AssetInfo::Token(WYND_CW20_ADDR.to_string()),
                    target_denom,
                ),
                JunoDestinationProject::WyndLP {
                    contract_address,
                    bonding_period,
                } => {
                    // get the pool info for the lp we're about to enter
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
                            pool_info.liquidity_token,
                            &cw20::Cw20QueryMsg::Balance {
                                address: target_address.to_string(),
                            },
                        )?
                    )},
            } },
        )
        .collect::<Result<Vec<_>, ContractError>>()
        .map(|msgs_list| msgs_list.into_iter().flatten().collect())?;

    all_msgs.append(&mut compounding_msgs);

    Ok(all_msgs)
}

pub fn wynd_token_swap(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: AssetInfo,
    target_denom: AssetInfo,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    Ok(create_wyndex_swap_msg(
        &target_address,
        comp_token_amount,
        staking_denom,
        target_denom,
        WYND_MULTI_HOP_ADDR.to_string(),
    )?)
}

pub fn neta_staking_msgs(
    target_address: Addr,
    (
        SimulationResponse {
            return_amount: expected_neta,
            ..
        },
        operations,
    ): (SimulationResponse, Vec<SwapOperation>),
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap juno for neta
    let neta_swap_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        WYND_CW20_ADDR.to_string(),
        &target_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: WYND_MULTI_HOP_ADDR.to_string(),
            amount: expected_neta,
            msg: to_binary(&wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
                operations,
                receiver: None,
                max_spread: None,
                minimum_receive: None,
                referral_address: None,
                referral_commission: None,
            })?,
        },
        None,
    )?);

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

    Ok(vec![neta_swap_msg, neta_stake_msg])
}

pub fn juno_staking_msgs(
    target_address: Addr,
    comp_token_amount: Uint128,
    validator_address: String,
    SimulationResponse {
        return_amount: expected_juno,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap wynd for juno
    let wynd_swap_msg = wynd_pair_swap_msg(
        &target_address,
        Asset {
            info: AssetInfo::Token(WYND_CW20_ADDR.to_string()),
            amount: comp_token_amount,
        },
        AssetInfo::Native("ujuno".to_string()),
        JUNO_WYND_PAIR_ADDR.to_string(),
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

    Ok(vec![wynd_swap_msg, juno_stake_msg])
}

#[allow(clippy::too_many_arguments)]
pub fn join_wynd_pool_msgs(
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

    // checks the number of assets in the pool. expected to be 2
    let asset_count: u128 = pool_info.asset_infos.len().try_into().unwrap();

    // calculates the amount of wynd to be swapped for each asset in the pool
    let wynd_amount_per_asset: Uint128 =
        comp_token_amount.checked_div_floor((asset_count, 1u128))?;

    // calculates the amount of each asset in the pool to be swapped for wynd
    let pool_assets = wynd_lp_asset_swaps(
        querier,
        &staking_denom,
        &wynd_amount_per_asset,
        &pool_info,
        &target_address,
    )?;

    // gathers the swap messages  from the WyndAssetLPMessages
    let mut swap_msgs: Vec<CosmosProtoMsg> = wynd_join_pool_msgs(
        target_address.to_string(),
        pool_contract_address,
        pool_assets,
    )?;

    // if the user already has lp tokens, we need to delegate them to the staking contract
    if !existing_lp_tokens.balance.is_zero() {
        swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
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

    Ok(swap_msgs)
    // will need to update things to utilize the routes from the factory
    // wyndex::factory::ROUTE;
}

/// Generates the wyndex swap messages and IncreaseAllowance (for cw20) messages that are needed before the actual pool can be entered.
/// These messages should ensure that we have the correct amount of assets in the pool contract
pub fn wynd_lp_asset_swaps(
    querier: &QuerierWrapper,
    staking_denom: &String,
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
