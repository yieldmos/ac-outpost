use cosmwasm_std::{Addr, QuerierWrapper, StdResult};

use osmosis_std::types::{
    cosmos::base::v1beta1::Coin,
    osmosis::poolmanager::v1beta1::{
        EstimateSwapExactAmountInRequest, EstimateSwapExactAmountInResponse,
        EstimateSwapExactAmountOutRequest, EstimateSwapExactAmountOutResponse,
        MsgSwapExactAmountIn, MsgSwapExactAmountOut, SwapAmountInRoute, SwapAmountOutRoute,
    },
};

use outpost_utils::msg_gen::CosmosProtoMsg;
use swaprouter::msg::GetRouteResponse;

const SWAPROUTER_ADDRESS: &str = "osmo1fy547nr4ewfc38z73ghr6x62p7eguuupm66xwk8v8rjnjyeyxdqs6gdqx7";

/// Queries the swaprouter's state to get a valid route from `from_denom` to `to_denom`
pub fn query_swap_in_routes(
    querier: &QuerierWrapper,
    from_token: &String,
    // just for error reporting purposes
    to_denom: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    let route_response: GetRouteResponse = querier.query_wasm_smart(
        SWAPROUTER_ADDRESS.to_string(),
        &swaprouter::msg::QueryMsg::GetRoute {
            input_denom: from_token.clone(),
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
    from_token: &String,
    // just for error reporting purposes
    to_denom: String,
) -> StdResult<Vec<SwapAmountOutRoute>> {
    let route_response: GetRouteResponse = querier.query_wasm_smart(
        SWAPROUTER_ADDRESS.to_string(),
        &swaprouter::msg::QueryMsg::GetRoute {
            input_denom: from_token.clone(),
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
    delegator_address: &Addr,
    from_denom: String,
    to_token: Coin,
) -> StdResult<(EstimateSwapExactAmountOutResponse, Vec<SwapAmountOutRoute>)> {
    if from_denom == to_token.denom {
        return Ok((
            EstimateSwapExactAmountOutResponse {
                token_in_amount: to_token.amount,
            },
            vec![],
        ));
    }

    let swap_route: Vec<SwapAmountOutRoute> =
        query_swap_out_routes(querier, &from_denom.clone(), to_token.denom.clone())?;

    let estimate = EstimateSwapExactAmountOutRequest {
        sender: delegator_address.to_string(),
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
    delegator_address: &Addr,
    from_token: Coin,
    // just for error reporting purposes
    to_denom: String,
) -> StdResult<(EstimateSwapExactAmountInResponse, Vec<SwapAmountInRoute>)> {
    let swap_route: Vec<SwapAmountInRoute> =
        query_swap_in_routes(querier, &from_token.denom.clone(), to_denom.clone())?;

    let estimate = EstimateSwapExactAmountInRequest {
        sender: delegator_address.to_string(),
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
    )?;

    let swap_msg = CosmosProtoMsg::OsmosisSwapExactAmountIn(MsgSwapExactAmountIn {
        token_in: Some(from_token),

        sender: delegator_address.to_string(),
        routes,
        token_out_min_amount: simulation.clone().token_out_amount,
    });

    Ok((simulation, vec![swap_msg]))
}
