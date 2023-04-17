use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, cosmwasm::wasm::v1::MsgExecuteContract};
use cosmwasm_std::{to_binary, Addr, Decimal, QuerierWrapper, StdError, Uint128};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
use wyndex::{
    asset::{Asset, AssetInfo, AssetValidated},
    pair::SimulationResponse,
};

use crate::errors::WyndHelperError;

/// Queries the Wyndex pool for the amount of `to_denom` that can be received for `from_token`
/// IMPORTANT: you must provide the pair contract address for the simulation
pub fn query_wynd_pool_swap(
    querier: &QuerierWrapper,
    pool_address: String,
    from_token: &Asset,
    // just for error reporting purposes
    to_denom: String,
) -> Result<SimulationResponse, WyndHelperError> {
    wyndex::querier::simulate(querier, pool_address, from_token).map_err(|_| {
        WyndHelperError::SwapSimulationError {
            from: from_token.info.to_string(),
            to: to_denom,
        }
    })
}

/// Queries the Wyndex multihop factory for the amount of `to_denom`
/// that can be received for a bunch of different tokens. This can be used
/// to compare the value of all the input offer tokens.
pub fn simulate_multiple_swaps(
    querier: &QuerierWrapper,
    offer_tokens: Vec<AssetValidated>,
    target_token: &wyndex::asset::AssetInfoValidated,
    multihop_factory_addr: &String,
) -> Result<Vec<(AssetValidated, SimulationResponse)>, WyndHelperError> {
    offer_tokens
        .into_iter()
        .map(|offer_token| {
            let simulation: SimulationResponse = querier
                .query_wasm_smart(
                    multihop_factory_addr,
                    &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
                        offer_amount: offer_token.amount,
                        operations: vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
                            offer_asset_info: offer_token.info.clone().into(),
                            ask_asset_info: target_token.clone().into(),
                        }],
                        referral: false,
                        referral_commission: None,
                    },
                )
                .map_err(|_| WyndHelperError::SwapSimulationError {
                    from: offer_token.info.to_string(),
                    to: target_token.to_string(),
                })?;

            Ok((offer_token.clone(), simulation))
        })
        .collect::<Result<Vec<_>, _>>()
}

pub fn create_wyndex_swap_operations(
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
) -> wyndex_multi_hop::msg::ExecuteMsg {
    let operations = vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
        offer_asset_info: offer_asset.clone(),
        ask_asset_info,
    }];
    wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        receiver: None,
        max_spread: Some(Decimal::percent(1)),
        referral_address: None,
        referral_commission: None,
    }
}

/// Creates a MsgExecuteContract for doing a token swap on Wyndex via the multihop router.
/// If you need to get a simulation of the swap as well, use `create_wyndex_swap_msg_and_simulation` instead
pub fn create_wyndex_swap_msg(
    sender: &Addr,
    offer_amount: Uint128,
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
    multihop_address: String,
) -> Result<Vec<CosmosProtoMsg>, StdError> {
    // no swap to do because the offer and ask tokens are the same
    if offer_asset.eq(&ask_asset_info) {
        return Ok(vec![]);
    }

    // the swap operations to be used by the multihop router
    let swap_ops = create_wyndex_swap_operations(offer_asset.clone(), ask_asset_info);

    match offer_asset {
        AssetInfo::Native(offer_denom) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            // multihop swap message when going from a native token
            create_exec_contract_msg(
                multihop_address,
                sender,
                &swap_ops,
                Some(vec![Coin {
                    amount: offer_amount.to_string(),
                    denom: offer_denom,
                }]),
            )?,
        )]),
        AssetInfo::Token(ask_token_contract_address) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            // multihop swap message when going from a cw20 token
            create_exec_contract_msg(
                ask_token_contract_address,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: multihop_address.to_string(),
                    amount: offer_amount,
                    msg: to_binary(&swap_ops)?,
                },
                None,
            )?,
        )]),
    }
}

/// Creates a MsgExecuteContract for doing a token swap on Wyndex via the multihop router
/// also returning the simulated resultant token amount
pub fn create_wyndex_swap_msg_with_simulation(
    querier: &QuerierWrapper,
    sender: &Addr,
    offer_amount: Uint128,
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
    multihop_address: String,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), StdError> {
    // no swap to do because the offer and ask tokens are the same
    if offer_asset.eq(&ask_asset_info) {
        return Ok((vec![], offer_amount));
    }

    // generate the operations for the multihop here that way we can use the same ops for
    // the simulation and the actual swap msg
    let swap_ops = create_wyndex_swap_operations(offer_asset.clone(), ask_asset_info);

    let simulated_swap: wyndex::pair::SimulationResponse;

    if let wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations { operations, .. } =
        swap_ops.clone()
    {
        simulated_swap = querier.query_wasm_smart(
            multihop_address.to_string(),
            &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
                offer_amount,
                operations,
                referral: false,
                referral_commission: None,
            },
        )?;
    } else {
        return Err(StdError::generic_err("Could not simulate swap operations"));
    }

    let exec: MsgExecuteContract;
    match offer_asset {
        AssetInfo::Native(offer_denom) => {
            // multihop swap message when going from a native token
            exec = create_exec_contract_msg(
                multihop_address,
                sender,
                &swap_ops,
                Some(vec![Coin {
                    amount: offer_amount.to_string(),
                    denom: offer_denom,
                }]),
            )?;
        }
        AssetInfo::Token(ask_token_contract_address) => {
            // multihop swap message when going from a cw20 token
            exec = create_exec_contract_msg(
                ask_token_contract_address,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: multihop_address.to_string(),
                    amount: offer_amount,
                    msg: to_binary(&swap_ops)?,
                },
                None,
            )?;
        }
    }
    Ok((
        vec![CosmosProtoMsg::ExecuteContract(exec)],
        simulated_swap.return_amount,
    ))
}

pub struct SwapSimResponse {
    pub swap_msgs: Vec<CosmosProtoMsg>,
    pub asset: AssetInfo,
    pub simulated_return_amount: Uint128,
}

/// Creates a MsgExecuteContract for doing multiple token swaps all with the same ask token
/// on Wyndex via the multihop router
/// also returning the simulated resultant token amounts
pub fn create_wyndex_swaps_with_sims(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
    offer_assets: Vec<AssetValidated>,
    ask_asset: AssetInfo,
    multihop_address: String,
) -> Result<SwapSimResponse, StdError> {
    let swaps_and_sims = offer_assets
        .into_iter()
        .map(|AssetValidated { info, amount }| {
            create_wyndex_swap_msg_with_simulation(
                querier,
                delegator_addr,
                amount,
                info.into(),
                ask_asset.clone(),
                multihop_address.to_string(),
            )
        })
        .collect::<Result<Vec<_>, StdError>>()?;

    let (swap_msgs, simulated_return_amount) = swaps_and_sims.into_iter().fold(
        (vec![], Uint128::zero()),
        |(mut swaps, mut sim_total), (swap, sim)| {
            swaps.extend(swap);
            sim_total += sim;
            (swaps, sim_total)
        },
    );

    Ok(SwapSimResponse {
        swap_msgs,
        asset: ask_asset,
        simulated_return_amount,
    })
}
