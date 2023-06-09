use std::iter;

use cosmos_sdk_proto::cosmos::{
    base::v1beta1::Coin, distribution::v1beta1::MsgWithdrawDelegatorReward,
    staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{to_binary, Addr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128};
use outpost_utils::{
    comp_prefs::{
        CompoundPrefs, DestinationAction, JunoDestinationProject, WyndLPBondingPeriod,
        WyndStakingBondingPeriod,
    },
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one},
    msg_gen::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
};

use wynd_helpers::{
    wynd_lp::{wynd_join_pool_msgs, WyndAssetLPMessages},
    wynd_swap::{create_wyndex_swap_msg_with_simulation, wynd_pair_swap_msg},
};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::{PairInfo, SimulationResponse},
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    queries::{self, query_juno_neta_swap, query_juno_wynd_swap},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};

pub const WYND_CW20_ADDR: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
pub const WYND_MULTI_HOP_ADDR: &str =
    "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w";
pub const JUNO_NETA_PAIR_ADDR: &str =
    "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra";
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
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let delegator: Addr = deps.api.addr_validate(&delegator_address)?;

    // validate that the user is authorized to compound
    let _ = is_authorized_compounder(
        deps.as_ref(),
        &info.sender,
        &delegator,
        ADMIN,
        AUTHORIZED_ADDRS,
    )?;

    // get the denom of the staking token. this should be "ujuno"
    let staking_denom = deps.querier.query_bonded_denom()?;

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let sub_msgs = prefs_to_msgs(
        &env.block.height,
        staking_denom.to_string(),
        &delegator,
        queries::query_pending_rewards(&deps.querier, &delegator, staking_denom)?,
        comp_prefs,
        deps.querier,
    )?;

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, sub_msgs)?;

    Ok(Response::default().add_message(exec_msg))
}

/// Converts the user's compound preferences into a list of CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    current_height: &u64,
    staking_denom: String,
    target_address: &Addr,
    AllPendingRewards {
        rewards: pending_rewards,
        total: total_rewards,
    }: AllPendingRewards,
    comp_prefs: CompoundPrefs,
    querier: QuerierWrapper,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // generate the withdraw rewards messages to grab all of the user's pending rewards
    // these should be the first msgs in the tx so the user has funds to compound
    let mut withdraw_rewards_msgs: Vec<CosmosProtoMsg> = pending_rewards
        .iter()
        .map(|PendingReward { validator, .. }| {
            CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
                validator_address: validator.to_string(),
                delegator_address: target_address.to_string(),
            })
        })
        .collect();

    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, JunoDestinationProject::JunoStaking { validator_address: "juno1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Result<Vec<CosmosProtoMsg>, ContractError> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<Vec<CosmosProtoMsg>, ContractError> { match destination {
                JunoDestinationProject::JunoStaking { validator_address } => {
                    Ok(vec![CosmosProtoMsg::Delegate(MsgDelegate {
                        validator_address,
                        amount: Some(Coin {
                            denom: total_rewards.denom.clone(),
                            amount: comp_token_amount.into(),
                        }),
                        delegator_address: target_address.to_string(),
                    })])
                },
                JunoDestinationProject::NetaStaking {} => neta_staking_msgs(
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    query_juno_neta_swap(&querier,comp_token_amount)?
                ),
                JunoDestinationProject::WyndStaking { bonding_period } =>
                 wynd_staking_msgs(
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    bonding_period,
                    query_juno_wynd_swap(&querier, comp_token_amount)?
                ),
                JunoDestinationProject::TokenSwap { target_denom } => wynd_helpers::wynd_swap::create_wyndex_swap_msg(
                    &target_address,
                    comp_token_amount,
                    AssetInfo::Native(staking_denom.clone()),
                    target_denom,
                    WYND_MULTI_HOP_ADDR.to_string(),
                )
                .map_err(|err| ContractError::Std(err)),
                JunoDestinationProject::WyndLP {
                    contract_address,
                    bonding_period,
                } => {

                    // fetch the pool info so that we know how to do the swaps for entering the lp
                    let pool_info: wyndex::pair::PairInfo = querier.query_wasm_smart(
                        contract_address.to_string(),
                        &wyndex::pair::QueryMsg::Pair {},
                    )?;

                    join_wynd_pool_msgs(
                        current_height,
                        &querier,
                        target_address.clone(),
                        comp_token_amount,
                        staking_denom.clone(),
                        contract_address,
                        bonding_period,
                         pool_info.clone(),
                         // checking the balance of the liquidity token to see if the user is already in the pool
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
        .map(|msgs_list| msgs_list.into_iter().flatten().collect());

    withdraw_rewards_msgs.append(&mut compounding_msgs?);

    Ok(withdraw_rewards_msgs)
}

pub fn neta_staking_msgs(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    SimulationResponse {
        return_amount: expected_neta,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap juno for neta
    let neta_swap_msg = wynd_pair_swap_msg(
        &target_address,
        Asset {
            info: AssetInfo::Native(staking_denom.clone()),
            amount: comp_token_amount,
        },
        AssetInfo::Token(NETA_CW20_ADDR.to_string()),
        JUNO_NETA_PAIR_ADDR.to_string(),
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

    Ok(vec![neta_swap_msg, neta_stake_msg])
}

pub fn wynd_staking_msgs(
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    bonding_period: WyndStakingBondingPeriod,
    SimulationResponse {
        return_amount: expected_wynd,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap juno for wynd
    let wynd_swap_msg = wynd_pair_swap_msg(
        &target_address,
        Asset {
            info: AssetInfo::Native(staking_denom.clone()),
            amount: comp_token_amount,
        },
        AssetInfo::Token(WYND_CW20_ADDR.to_string()),
        JUNO_WYND_PAIR_ADDR.to_string(),
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

    Ok(vec![wynd_swap_msg, wynd_stake_msg])
}

#[allow(clippy::too_many_arguments)]
fn join_wynd_pool_msgs(
    _current_height: &u64,
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

    // check the number of assets in the pool, but realistically this is expected to be 2
    let asset_count: u128 = pool_info.asset_infos.len().try_into().unwrap();

    // the amount of juno that will be used to swap for each asset in the pool
    let juno_amount_per_asset: Uint128 =
        comp_token_amount.checked_div_floor((asset_count, 1u128))?;

    // the list of prepared swaps and assets that will be used to join the pool
    let pool_assets = wynd_lp_asset_swaps(
        querier,
        &staking_denom,
        &juno_amount_per_asset,
        &pool_info,
        &target_address,
    )?;

    // the final list of swap messages that need to be executed before joining the pool is possible
    let mut swap_msgs: Vec<CosmosProtoMsg> = wynd_join_pool_msgs(
        target_address.to_string(),
        pool_contract_address,
        pool_assets,
    )?;

    // as a temporary measure we bond the existing unbonded lp tokens- this is should be resolved when wyndex updates itself
    // to add a bonding simulate function
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
