use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{to_json_binary, Addr, QuerierWrapper, StdError, Uint128};
use cw_grant_spec::grants::{GrantBase, GrantRequirement};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
use white_whale::pool_network::{
    asset::{Asset, AssetInfo},
    pair::{ExecuteMsg as PairExecuteMsg, SimulationResponse},
    router::{ExecuteMsg, SimulateSwapOperationsResponse, SwapOperation},
};

// /// Queries the Wyndex pool for the amount of `to_denom` that can be received for `from_token`
// /// IMPORTANT: you must provide the pair contract address for the simulation
// pub fn simulate_wynd_pool_swap(
//     querier: &QuerierWrapper,
//     pool_address: &str,
//     from_token: &Asset,
//     // just for error reporting purposes
//     to_denom: String,
// ) -> Result<SimulationResponse, WyndHelperError> {

//     wyndex::querier::simulate(querier, pool_address, from_token).map_err(|_| {
//         WyndHelperError::SwapSimulationError {
//             from: from_token.info.to_string(),
//             to: to_denom,
//         }
//     })
// }

// /// Generates the messages for swapping a token on Wyndex via a given pair contract
// pub fn wynd_pair_swap_msg(
//     sender: &Addr,
//     offer_asset: Asset,
//     ask_asset: AssetInfo,
//     pair_contract_address: &str,
// ) -> Result<CosmosProtoMsg, TerraswapHelperError> {
//     let swap_msg = match offer_asset.info.clone() {
//         AssetInfo::Native(denom) => {
//             // swap message when going from a native token
//             CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
//                 pair_contract_address,
//                 sender,
//                 &wyndex::pair::ExecuteMsg::Swap {
//                     offer_asset: offer_asset.clone(),
//                     ask_asset_info: Some(ask_asset),
//                     max_spread: None,
//                     belief_price: None,
//                     to: None,
//                     referral_address: None,
//                     referral_commission: None,
//                 },
//                 Some(vec![Coin {
//                     denom,
//                     amount: offer_asset.amount.to_string(),
//                 }]),
//             )?)
//         }
//         AssetInfo::Token(offer_token_address) => {
//             // swap message when going from a cw20 token
//             CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
//                 offer_token_address,
//                 sender,
//                 &cw20::Cw20ExecuteMsg::Send {
//                     contract: pair_contract_address.to_owned(),
//                     amount: offer_asset.amount,
//                     msg: to_binary(&wyndex::pair::Cw20HookMsg::Swap {
//                         ask_asset_info: Some(ask_asset),
//                         belief_price: None,
//                         max_spread: None,
//                         to: None,
//                         referral_address: None,
//                         referral_commission: None,
//                     })?,
//                 },
//                 None,
//             )?)
//         }
//     };

//     Ok(swap_msg)
// }

// /// Generates the messages for swapping a token on Wyndex via a given pair contract
// /// also returns the swap simulationso that it can be used for subsequent calculations
// pub fn simulate_and_swap_wynd_pair(
//     querier: &QuerierWrapper,
//     sender: &Addr,
//     pair_contract_address: &str,
//     offer_asset: Asset,
//     ask_asset: AssetInfo,
// ) -> Result<(CosmosProtoMsg, SimulationResponse), TerraswapHelperError> {
//     let simulation = simulate_wynd_pool_swap(
//         querier,
//         pair_contract_address,
//         &offer_asset,
//         ask_asset.to_string(),
//     )?;

//     let swap_msg = wynd_pair_swap_msg(sender, offer_asset, ask_asset, pair_contract_address)?;

//     Ok((swap_msg, simulation))
// }

/// Queries the terraswap pool for the amount of `to_denom` that can be received for `from_token`
/// IMPORTANT: you must provide the pair contract address for the simulation
pub fn simulate_pool_swap(
    querier: &QuerierWrapper,
    pool_address: &str,
    from_token: &Asset,
) -> Result<SimulationResponse, StdError> {
    let simulated_swap: SimulationResponse = querier.query_wasm_smart(
        pool_address.to_string(),
        &white_whale::pool_network::pair::QueryMsg::Simulation {
            offer_asset: from_token.clone(),
        },
    )?;

    Ok(simulated_swap)
}

/// Creates a MsgExecuteContract for doing a token swap on terraswap via the multihop router.
/// If you need to get a simulation of the swap as well, use `create_swap_msg_and_simulation` instead
pub fn create_swap_msg(
    sender: &Addr,
    offer_amount: Uint128,
    swap_routes: Vec<SwapOperation>,
    multihop_address: String,
) -> Result<Vec<CosmosProtoMsg>, StdError> {
    // // no swap to do because the offer and ask tokens are the same
    // if swap_route.offer_asset_info.eq(&swap_route.ask_asset_info) {
    //     return Ok(vec![]);
    // }

    let swap_ops = ExecuteMsg::ExecuteSwapOperations {
        operations: swap_routes.clone(),
        minimum_receive: None,
        to: None,
    };

    match swap_routes.first() {
        Some(SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken { denom },
            ..
        }) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            // multihop swap message when going from a native token
            create_exec_contract_msg(
                multihop_address,
                sender,
                &swap_ops,
                Some(vec![Coin {
                    amount: offer_amount.to_string(),
                    denom: denom.to_string(),
                }]),
            )?,
        )]),
        Some(SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token { contract_addr },
            ..
        }) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            // multihop swap message when going from a cw20 token
            create_exec_contract_msg(
                contract_addr,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: multihop_address.to_string(),
                    amount: offer_amount,
                    msg: to_json_binary(&swap_ops)?,
                },
                None,
            )?,
        )]),
        // a problem occurred here
        _ => Ok(vec![]),
    }
}

/// Creates a MsgExecuteContract for doing a token swap on terraswap via the multihop router
/// also returning the simulated resultant token amount
pub fn create_terraswap_swap_msg_with_simulation(
    querier: &QuerierWrapper,
    sender: &Addr,
    offer_amount: Uint128,
    swap_routes: Vec<SwapOperation>,
    multihop_address: String,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), StdError> {
    // // no swap to do because the offer and ask tokens are the same
    // if offer_asset.eq(&ask_asset_info) {
    //     return Ok((vec![], offer_amount));
    // }

    // // generate the operations for the multihop here that way we can use the same ops for
    // // the simulation and the actual swap msg
    // let swap_ops = create_wyndex_swap_operations(offer_asset.clone(), ask_asset_info);

    let simulated_swap: SimulateSwapOperationsResponse = querier.query_wasm_smart(
        multihop_address.to_string(),
        &white_whale::pool_network::router::QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations: swap_routes.clone(),
        },
    )?;

    let exec = create_swap_msg(sender, offer_amount, swap_routes, multihop_address)?;

    Ok((exec, simulated_swap.amount))
}

/// Queries a specific terraswap pool and returns the swap message as well as simulated swap amount
pub fn create_terraswap_pool_swap_msg_with_simulation(
    querier: &QuerierWrapper,
    sender: &Addr,
    offer_asset: Asset,
    pool_address: &Addr,
) -> Result<(CosmosProtoMsg, Uint128), StdError> {
    // // no swap to do because the offer and ask tokens are the same
    // if offer_asset.eq(&ask_asset_info) {
    //     return Ok((vec![], offer_amount));
    // }

    let simulated_swap: SimulationResponse =
        simulate_pool_swap(querier, &pool_address.to_string(), &offer_asset)?;

    let swap_msg = create_terraswap_pool_swap_msg(sender, offer_asset, pool_address)?;

    Ok((swap_msg, simulated_swap.return_amount))
}

pub fn create_terraswap_pool_swap_msg(
    sender: &Addr,
    offer_asset: Asset,
    pool_address: &Addr,
) -> Result<CosmosProtoMsg, StdError> {
    // // no swap to do because the offer and ask tokens are the same
    // if offer_asset.eq(&ask_asset_info) {
    //     return Ok((vec![], offer_amount));
    // }

    Ok(match offer_asset.clone() {
        Asset {
            info: AssetInfo::NativeToken { denom },
            amount,
        } =>
        // swap the native asset
        {
            CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                pool_address,
                sender,
                &PairExecuteMsg::Swap {
                    offer_asset,
                    belief_price: None,
                    max_spread: None,
                    to: None,
                },
                Some(vec![Coin {
                    denom,
                    amount: amount.to_string(),
                }]),
            )?)
        }

        Asset {
            info: AssetInfo::Token { contract_addr },
            amount,
        } =>
        // call send on the cw20 contract and have it respond with a swap
        {
            CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                contract_addr,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: pool_address.to_string(),
                    amount,
                    msg: to_json_binary(&PairExecuteMsg::Swap {
                        offer_asset,
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    })?,
                },
                None,
            )?)
        }
    })
}

/// Generates the grant spec for doing a swap via the terraswap multihop for a native token
pub fn terraswap_multihop_swap_grant(
    base: GrantBase,
    multihop_addr: Addr,
    offer_denom: &str,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        multihop_addr,
        vec!["execute_swap_operations"],
        Some(offer_denom),
    )]
}

/// Generates the grant spec for doing a swap via the terraswap multihop for a cw20 token
pub fn terraswap_cw20_multihop_swap_grant(
    base: GrantBase,
    cw20_addr: Addr,
) -> Vec<GrantRequirement> {
    vec![GrantRequirement::default_contract_exec_auth(
        base,
        cw20_addr,
        vec!["send"],
        None,
    )]
}

pub fn terraswap_multihop_grant(
    base: GrantBase,
    multihop_addr: Addr,
    offer_asset: AssetInfo,
) -> Vec<GrantRequirement> {
    match offer_asset {
        AssetInfo::NativeToken { denom } => {
            terraswap_multihop_swap_grant(base, multihop_addr, &denom)
        }
        AssetInfo::Token { contract_addr } => {
            terraswap_cw20_multihop_swap_grant(base, Addr::unchecked(contract_addr))
        }
    }
}
