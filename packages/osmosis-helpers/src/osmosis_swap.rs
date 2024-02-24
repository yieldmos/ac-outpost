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

use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use outpost_utils::{helpers::DestProjectMsgs, msg_gen::CosmosProtoMsg};

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

// /// Queries the swaprouter's state to get a valid route from `from_denom` to `to_denom`
// pub fn query_swap_in_routes(
//     querier: &QuerierWrapper,
//     from_token: &str,
//     // just for error reporting purposes
//     to_denom: String,
//     swap_router_address: String,
// ) -> StdResult<Vec<SwapAmountInRoute>> {
//     let route_response: GetRouteResponse = querier.query_wasm_smart(
//         swap_router_address,
//         &GetRoute {
//             input_denom: from_token.to_string(),
//             output_denom: to_denom,
//         },
//     )?;

//     let route: Vec<SwapAmountInRoute> = route_response
//         .pool_route
//         .iter()
//         .map(|route| SwapAmountInRoute {
//             pool_id: route.pool_id,
//             token_out_denom: route.token_out_denom.clone(),
//         })
//         .collect();

//     Ok(route)
// }

// /// Queries the swaprouter's state to get a valid route from `from_denom` to `to_denom`
// pub fn query_swap_out_routes(
//     querier: &QuerierWrapper,
//     from_token: &str,
//     // just for error reporting purposes
//     to_denom: String,
//     swap_router_address: String,
// ) -> StdResult<Vec<SwapAmountOutRoute>> {
//     let route_response: GetRouteResponse = querier.query_wasm_smart(
//         swap_router_address,
//         &GetRoute {
//             input_denom: from_token.to_string(),
//             output_denom: to_denom,
//         },
//     )?;

//     let route: Vec<SwapAmountOutRoute> = route_response
//         .pool_route
//         .iter()
//         .map(|route| SwapAmountOutRoute {
//             pool_id: route.pool_id,
//             // this is likely wrong and shouldn't be an out denom
//             token_in_denom: route.token_out_denom.clone(),
//         })
//         .collect();

//     Ok(route)
// }

// pub fn simulate_exact_out_swap(
//     querier: &QuerierWrapper,
//     _delegator_address: &Addr,
//     from_denom: String,
//     to_token: Coin,
//     swap_router_address: String,
// ) -> StdResult<(EstimateSwapExactAmountOutResponse, Vec<SwapAmountOutRoute>)> {
//     if from_denom == to_token.denom {
//         return Ok((
//             EstimateSwapExactAmountOutResponse {
//                 token_in_amount: to_token.amount,
//             },
//             vec![],
//         ));
//     }

//     let swap_route: Vec<SwapAmountOutRoute> = query_swap_out_routes(
//         querier,
//         &from_denom.clone(),
//         to_token.denom.clone(),
//         swap_router_address,
//     )?;

//     let estimate = EstimateSwapExactAmountOutRequest {
//         // sender: delegator_address.to_string(),
//         pool_id: swap_route.clone().first().unwrap().pool_id,
//         token_out: to_token.denom,
//         routes: swap_route.clone(),
//     }
//     .query(querier)?;

//     Ok((estimate, swap_route))
// }

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

/// Generated a route for a swap the resolves to a `TargetAsset` which
/// has a given pool id and paired pool asset. for example (ION -> DYDX (pool#1246, paired with USDC))
pub fn generate_known_to_unknown_route(
    denoms: Denoms,
    pools: DestProjectMsgs,
    from_denom: String,
    TargetAsset {
        denom: to_denom,
        exit_pool_id,
        paired_asset,
    }: TargetAsset,
) -> StdResult<Vec<SwapAmountInRoute>> {
    // nothing to swap if from and to are the same denom
    if from_denom.eq(&to_denom) {
        Ok(vec![])
    }

    // if the from isn't a known denom this isn't the right place to get a route
    if !denoms.is_known_denom(from_denom) {
        Err(InvalidRouteDenom {
            denom: to_denom.as_str(),
            label: "known to unknown from denom",
        })
    }

    match paired_asset {
        // if the target asset is paired with osmo and our from denom is osmo we have our route
        OSMO if from_denom.eq(&denoms.osmo) => Ok(vec![SwapAmountInRoute {
            pool_id: exit_pool_id,
            token_out_denom: to_denom,
        }]),
        // if the target asset is paired with usdc and our from denom is usdc we have our route
        USDC if from_denom.eq(&denoms.usdc) => Ok(vec![SwapAmountInRoute {
            pool_id: exit_pool_id,
            token_out_denom: to_denom,
        }]),
        OSMO => {
            let mut to_osmo_route =
                generate_known_to_known_route(denoms, pools, from_denom, denoms.osmo)?;
            to_osmo_route.push(SwapAmountInRoute {
                pool_id: exit_pool_id,
                token_out_denom: to_denom,
            });
            Ok(to_osmo_route)
        }
        USDC => {
            let mut to_usdc_route =
                generate_known_to_known_route(denoms, pools, from_denom, denoms.usdc)?;
            to_usdc_route.push(SwapAmountInRoute {
                pool_id: exit_pool_id,
                token_out_denom: to_denom,
            });
            Ok(to_usdc_route)
        }
    }
}

/// Generates a route for a swap from a known denom to another known denom.
/// Known denoms denote anything in the `OsmoPools` or `UsdcPools` structs
pub fn generate_known_to_known_route(
    denoms: Denoms,
    pools: DestProjectSwapRoutes,
    from_denom: String,
    to_denom: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    // nothing to swap if from and to are the same denom
    if from_denom.eq(&to_denom) {
        Ok(vec![])
    }

    // validate that from and to are both known denoms
    if !denoms.is_known_denom(from_denom) {
        Err(InvalidRouteDenom {
            denom: from_denom.as_str(),
            label: "known to known from denom",
        })
    }
    if !denoms.is_known_denom(to_denom) {
        Err(InvalidRouteDenom {
            denom: to_denom.as_str(),
            label: "known to known to denom",
        })
    }

    match (
        pools.osmo.get_pool_id(from_denom),
        pools.osmo.get_pool_id(to_denom),
        pools.usdc.get_pool_id(from_denom),
        pools.usdc.get_pool_id(to_denom),
    ) {
        // special case where we're going to or from osmo and there's an osmo pooll
        (Some(pool_id), _, _, _) | (_, Some(pool_id), _, _)
            if to_denom.eq(&denoms.osmo) || from_denom.eq(&denoms.osmo) =>
        {
            Ok(vec![SwapAmountInRoute {
                pool_id,
                token_out_denom: to_denom,
            }])
        }
        // special case where we're going to usdc and there's a usdc pool
        (_, _, Some(pool_id), _) | (_, _, _, Some(pool_id))
            if to_denom.eq(&denoms.usdc) || from_denom.eq(&denoms.usdc) =>
        {
            Ok(vec![SwapAmountInRoute {
                pool_id,
                token_out_denom: to_denom,
            }])
        }
        // can swap via osmo (for example MBRN -> OSMO -> WHALE)
        (Some(in_pool_id), Some(out_pool_id), _, _) => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.osmo,
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom,
            },
        ]),
        // can swap via usdc (for example TIA -> USDC -> CDT)
        (_, _, Some(in_pool_id), Some(out_pool_id)) => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.usdc,
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom,
            },
        ]),

        // can swap to osmo and then to usdc and then out (for example MARS -> OSMO -> USDC -> axlUSDC)
        (Some(in_pool_id), _, _, Some(out_pool_id)) => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.osmo,
            },
            SwapAmountInRoute {
                pool_id: pools.osmo.usdc,
                token_out_denom: denoms.usdc,
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom,
            },
        ]),

        // can swap to usdc and then to osmo and then out (for example axlUSDC -> USDC -> OSMO -> ION)
        (_, Some(out_pool_id), Some(in_pool_id), _) => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.usdc,
            },
            SwapAmountInRoute {
                pool_id: pools.usdc.osmo,
                token_out_denom: denoms.osmo,
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom,
            },
        ]),
        _ => Err(NoKnownToKnownRoute {
            from_denom,
            to_denom,
        }),
    }
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
    route: Vec<SwapAmountOutRoute>,
) -> StdResult<(EstimateSwapExactAmountInResponse, Vec<SwapAmountInRoute>)> {
    let estimate = EstimateSwapExactAmountInRequest {
        // sender: delegator_address.to_string(),
        pool_id: swap_route.clone().first().unwrap().pool_id,
        token_in: from_token.denom,
        routes: route.clone(),
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

    // TODO: set route here
    let route = vec![];

    let (simulation, routes) = simulate_swap(
        querier,
        delegator_address,
        from_token.clone(),
        to_denom.clone(),
        swap_router_address,
        route,
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
    offer_asset: cosmwasm_std::Coin,
    token_out_denom: &str,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), OsmosisHelperError> {
    let offer_coin = OsmosisCoin {
        denom: offer_asset.denom.to_string(),
        amount: offer_asset.amount.to_string(),
    };

    Ok((
        vec![CosmosProtoMsg::OsmosisSwapExactAmountIn(
            MsgSwapExactAmountIn {
                token_in: Some(offer_coin.clone()),
                sender: user_addr.to_string(),
                token_out_min_amount: "0".to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: *pool_id,
                    token_out_denom: token_out_denom.to_string(),
                }],
            },
        )],
        Uint128::from_str(
            simulate_pool_swap(querier, pool_id, &offer_coin, token_out_denom)?
                .token_out_amount
                .as_str(),
        )?,
    ))
}
