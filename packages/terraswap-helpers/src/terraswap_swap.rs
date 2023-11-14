use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{to_json_binary, Addr, QuerierWrapper, StdError, Uint128};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
use white_whale::pool_network::{
    asset::AssetInfo,
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
