use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};

use osmosis_std::types::{
    cosmos::base::v1beta1::Coin,
    osmosis::poolmanager::v1beta1::{
        EstimateSwapExactAmountInRequest, EstimateSwapExactAmountInResponse,
        EstimateSwapExactAmountOutRequest, EstimateSwapExactAmountOutResponse,
        MsgSwapExactAmountIn, MsgSwapExactAmountOut, SwapAmountInRoute, SwapAmountOutRoute,
    },
};

use outpost_utils::msg_gen::CosmosProtoMsg;

use crate::errors::OsmosisHelperError;

// TODO: this should come from the swaprouter module instead of being copy and pasted here
#[cw_serde]
pub struct GetRouteResponse {
    pub pool_route: Vec<SwapAmountInRoute>,
}

// TODO: this should come from the swaprouter module instead of being copy and pasted here
#[cw_serde]
pub struct GetRoute {
    input_denom: String,
    output_denom: String,
}

/// Queries the swaprouter's state to get a valid route from `from_denom` to `to_denom`
pub fn query_swap_in_routes(
    querier: &QuerierWrapper,
    from_token: &str,
    // just for error reporting purposes
    to_denom: String,
    swap_router_address: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    let route_response: GetRouteResponse = querier.query_wasm_smart(
        swap_router_address,
        &GetRoute {
            input_denom: from_token.to_string(),
            output_denom: to_denom,
        },
    )?;

    let route: Vec<SwapAmountInRoute> = route_response
        .pool_route
        .iter()
        .map(|route| SwapAmountInRoute {
            pool_id: route.pool_id,
            token_out_denom: route.token_out_denom.clone(),
        })
        .collect();

    Ok(route)
}

/// Queries the swaprouter's state to get a valid route from `from_denom` to `to_denom`
pub fn query_swap_out_routes(
    querier: &QuerierWrapper,
    from_token: &str,
    // just for error reporting purposes
    to_denom: String,
    swap_router_address: String,
) -> StdResult<Vec<SwapAmountOutRoute>> {
    let route_response: GetRouteResponse = querier.query_wasm_smart(
        swap_router_address,
        &GetRoute {
            input_denom: from_token.to_string(),
            output_denom: to_denom,
        },
    )?;

    let route: Vec<SwapAmountOutRoute> = route_response
        .pool_route
        .iter()
        .map(|route| SwapAmountOutRoute {
            pool_id: route.pool_id,
            // this is likely wrong and shouldn't be an out denom
            token_in_denom: route.token_out_denom.clone(),
        })
        .collect();

    Ok(route)
}

pub fn simulate_exact_out_swap(
    querier: &QuerierWrapper,
    _delegator_address: &Addr,
    from_denom: String,
    to_token: Coin,
    swap_router_address: String,
) -> StdResult<(EstimateSwapExactAmountOutResponse, Vec<SwapAmountOutRoute>)> {
    if from_denom == to_token.denom {
        return Ok((
            EstimateSwapExactAmountOutResponse {
                token_in_amount: to_token.amount,
            },
            vec![],
        ));
    }

    let swap_route: Vec<SwapAmountOutRoute> = query_swap_out_routes(
        querier,
        &from_denom.clone(),
        to_token.denom.clone(),
        swap_router_address,
    )?;

    let estimate = EstimateSwapExactAmountOutRequest {
        // sender: delegator_address.to_string(),
        pool_id: swap_route.clone().first().unwrap().pool_id,
        token_out: to_token.denom,
        routes: swap_route.clone(),
    }
    .query(querier)?;

    Ok((estimate, swap_route))
}

pub fn generate_exact_out_swap_msg_from_sim(
    delegator_address: &Addr,
    from_denom: String,
    to_token: Coin,
    sim: EstimateSwapExactAmountOutResponse,
    routes: Vec<SwapAmountOutRoute>,
) -> StdResult<Vec<CosmosProtoMsg>> {
    if from_denom == to_token.denom {
        return Ok(vec![]);
    }

    let swap_msg = CosmosProtoMsg::OsmosisSwapExactAmountOut(MsgSwapExactAmountOut {
        token_out: Some(to_token),

        sender: delegator_address.to_string(),
        routes,
        token_in_max_amount: sim.clone().token_in_amount,
    });

    Ok(vec![swap_msg])
}

/// Queries the osmosis for the amount of `to_denom` that can be received for `from_token`
/// Returns both the swap simulation and the queried swap route
pub fn simulate_swap(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    from_token: Coin,
    // just for error reporting purposes
    to_denom: String,
    swap_router_address: String,
) -> StdResult<(EstimateSwapExactAmountInResponse, Vec<SwapAmountInRoute>)> {
    let swap_route: Vec<SwapAmountInRoute> = query_swap_in_routes(
        querier,
        &from_token.denom.clone(),
        to_denom.clone(),
        swap_router_address,
    )?;

    let estimate = EstimateSwapExactAmountInRequest {
        // sender: delegator_address.to_string(),
        pool_id: swap_route.clone().first().unwrap().pool_id,
        token_in: from_token.denom,
        routes: swap_route.clone(),
    }
    .query(querier)?;

    Ok((estimate, swap_route))
}

pub fn generate_swap_msg(
    querier: &QuerierWrapper,
    delegator_address: &Addr,
    from_token: Coin,
    to_denom: String,
    swap_router_address: String,
) -> StdResult<(EstimateSwapExactAmountInResponse, Vec<CosmosProtoMsg>)> {
    if from_token.denom == to_denom {
        return Ok((
            EstimateSwapExactAmountInResponse {
                token_out_amount: from_token.amount,
            },
            vec![],
        ));
    }

    let (simulation, routes) = simulate_swap(
        querier,
        delegator_address,
        from_token.clone(),
        to_denom.clone(),
        swap_router_address,
    )?;

    let swap_msg = CosmosProtoMsg::OsmosisSwapExactAmountIn(MsgSwapExactAmountIn {
        token_in: Some(from_token),

        sender: delegator_address.to_string(),
        routes,
        token_out_min_amount: simulation.clone().token_out_amount,
    });

    Ok((simulation, vec![swap_msg]))
}

pub fn simulate_pool_swap(
    querier: &QuerierWrapper,
    pool_id: &u64,
    offer_asset: &Coin,
    token_out_denom: &str,
) -> Result<EstimateSwapExactAmountInResponse, OsmosisHelperError> {
    let simulation = EstimateSwapExactAmountInRequest {
        pool_id: pool_id.clone(),
        token_in: offer_asset.denom.clone(),
        routes: vec![SwapAmountInRoute {
            pool_id: *pool_id,
            token_out_denom: token_out_denom.to_string(),
        }],
    }
    .query(querier)?;

    Ok(simulation)
}

pub fn pool_swap_with_sim(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    pool_id: &u64,
    offer_asset: Coin,
    token_out_denom: &str,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), OsmosisHelperError> {
    Ok((
        vec![CosmosProtoMsg::OsmosisSwapExactAmountIn(
            MsgSwapExactAmountIn {
                token_in: Some(offer_asset),
                sender: user_addr.to_string(),
                token_out_min_amount: "0".to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: *pool_id,
                    token_out_denom: token_out_denom.to_string(),
                }],
            },
        )],
        Uint128::from_str(
            simulate_pool_swap(querier, pool_id, &offer_asset, token_out_denom)?
                .token_out_amount
                .as_str(),
        )?,
    ))
}
