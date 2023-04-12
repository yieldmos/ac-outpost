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
    helpers::{calculate_compound_amounts, prefs_sum_to_one},
    msgs::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
};
use wyndex::{
    asset::{Asset, AssetInfo, AssetInfoValidated},
    pair::{PairInfo, SimulationResponse},
};

use crate::{
    contract::{AllPendingRewards, PendingReward},
    queries::{self, query_juno_neta_swap, query_juno_wynd_swap},
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
    _info: MessageInfo,
    delegator_address: String,
    comp_prefs: CompoundPrefs,
) -> Result<Response, ContractError> {
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    let delegator = deps.api.addr_validate(&delegator_address)?;
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

    // calculates the amount of ujuno that will be used for each target project accurately
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

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
                JunoDestinationProject::TokenSwap { target_denom } => wynd_token_swap(
                    target_address.clone(),
                    comp_token_amount,
                    staking_denom.clone(),
                    target_denom,
                ),
                JunoDestinationProject::WyndLP {
                    contract_address,
                    bonding_period,
                } => {

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

pub fn wynd_token_swap(
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
                WYND_MULTI_HOP_ADDR.to_string(),
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
    let neta_swap_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        JUNO_NETA_PAIR_ADDR.to_string(),
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
    let wynd_swap_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        JUNO_WYND_PAIR_ADDR.to_string(),
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
    current_height: &u64,
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
        current_height,
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
            pool_contract_address,
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

struct WyndAssetLPMessages {
    /// The msgs to perform the token swaps and if applicable the increase allowances
    swap_msgs: Vec<CosmosProtoMsg>,
    /// The asset denom and amount that will be sent to the pool contract
    target_asset_info: Asset,
}

/// Generates the wyndex swap messages and IncreaseAllowance (for cw20) messages that are needed before the actual pool can be entered.
/// These messages should ensure that we have the correct amount of assets in the pool contract
fn wynd_lp_asset_swaps(
    current_height: &u64,
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
        // map over each asset in the pool
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

                    // simulate the swap to know how much of the target asset we can expect to have
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
                                        WYND_MULTI_HOP_ADDR.to_string(),
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
                            Ok(WyndAssetLPMessages {
                                swap_msgs: vec![
                                    // the swap to get the cw20
                                    CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                        WYND_MULTI_HOP_ADDR.to_string(),
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
                                        cw20_token_addr.to_string(),
                                        &target_address,
                                        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                                            spender: pool_contract_address.to_string(),
                                            amount: swap_simulate.amount,
                                            expires: Some(cw20::Expiration::AtHeight(
                                                *current_height,
                                            )),
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
