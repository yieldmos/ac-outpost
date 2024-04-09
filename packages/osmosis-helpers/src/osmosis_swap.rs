use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, BlockInfo, Coin as CWCoin, Decimal, QuerierWrapper, StdResult, Storage, Timestamp,
    Uint128,
};

use cw_grant_spec::grants::{GrantBase, GrantRequirement};
use osmosis_destinations::{
    comp_prefs::{DestProjectSwapRoutes, KnownPairedPoolAsset, TargetAsset},
    pools::{Denoms, MultipleStoredPools, StoredDenoms},
};
use osmosis_std::types::{cosmos::base::v1beta1::Coin, osmosis::twap::v1beta1::TwapQuerier};
use osmosis_std::{
    shim,
    types::osmosis::{
        poolmanager::v1beta1::{
            EstimateSwapExactAmountInRequest, EstimateSwapExactAmountInResponse,
            EstimateSwapExactAmountOutResponse, MsgSwapExactAmountIn, MsgSwapExactAmountOut,
            SwapAmountInRoute, SwapAmountOutRoute,
        },
        twap::v1beta1::ArithmeticTwapToNowResponse,
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

/// Denotes the found known pools pertaining to a swap.
/// Since only osmo and usdc pools are premeditated those are the only pools to pass here
pub struct KnownRoutePools {
    pub from_token_osmo_pool: Option<u64>,
    pub to_token_osmo_pool: Option<u64>,
    pub from_token_usdc_pool: Option<u64>,
    pub to_token_usdc_pool: Option<u64>,
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

/// The base data needed to be able to construct routes
pub struct OsmosisRoutePools<'a> {
    pub stored_denoms: StoredDenoms<'a>,
    pub stored_pools: MultipleStoredPools<'a>,
    pub pools: DestProjectSwapRoutes,
    pub denoms: Denoms,
}

/// Generated a route for a swap the resolves to a `TargetAsset` which
/// has a given pool id and paired pool asset. for example (ION -> DYDX (pool#1246, paired with USDC))
pub fn generate_known_to_unknown_route(
    store: &dyn Storage,
    route_pools: OsmosisRoutePools,
    from_denom: &str,
    TargetAsset {
        denom: to_denom,
        exit_pool_id,
        paired_asset,
    }: TargetAsset,
) -> Result<Vec<SwapAmountInRoute>, OsmosisHelperError> {
    // nothing to swap if from and to are the same denom
    if from_denom.eq(&to_denom) {
        return Ok(vec![]);
    }

    let denoms = route_pools.denoms.clone();

    // if the from isn't a known denom this isn't the right place to get a route
    if !route_pools.stored_denoms.has(store, from_denom) {
        return Err(OsmosisHelperError::InvalidRouteDenom {
            denom: to_denom.to_string(),
            label: "known to unknown from denom".to_string(),
        });
    }

    match paired_asset {
        // if the target asset is paired with osmo and our from denom is osmo we have our route
        KnownPairedPoolAsset::OSMO if from_denom.eq(&denoms.osmo) => Ok(vec![SwapAmountInRoute {
            pool_id: exit_pool_id,
            token_out_denom: to_denom,
        }]),
        // if the target asset is paired with usdc and our from denom is usdc we have our route
        KnownPairedPoolAsset::USDC if from_denom.eq(&denoms.usdc) => Ok(vec![SwapAmountInRoute {
            pool_id: exit_pool_id,
            token_out_denom: to_denom,
        }]),
        KnownPairedPoolAsset::OSMO => {
            let mut to_osmo_route =
                generate_known_to_known_route(store, route_pools, from_denom, &denoms.osmo)?;
            to_osmo_route.push(SwapAmountInRoute {
                pool_id: exit_pool_id,
                token_out_denom: to_denom,
            });
            Ok(to_osmo_route)
        }
        KnownPairedPoolAsset::USDC => {
            let mut to_usdc_route =
                generate_known_to_known_route(store, route_pools, from_denom, &denoms.usdc)?;
            to_usdc_route.push(SwapAmountInRoute {
                pool_id: exit_pool_id,
                token_out_denom: to_denom,
            });
            Ok(to_usdc_route)
        }
    }
}

pub fn generate_known_to_known_route(
    store: &dyn Storage,
    OsmosisRoutePools {
        stored_denoms,
        stored_pools,
        pools,
        denoms,
    }: OsmosisRoutePools,
    from_denom: &str,
    to_denom: &str,
) -> Result<Vec<SwapAmountInRoute>, OsmosisHelperError> {
    // nothing to swap if from and to are the same denom
    if from_denom.eq(to_denom) {
        return Ok(vec![]);
    }

    // validate that from and to are both known denoms
    if !stored_denoms.has(store, &from_denom) {
        return Err(OsmosisHelperError::InvalidRouteDenom {
            denom: from_denom.to_string(),
            label: "known to known from denom".to_string(),
        });
    }
    if !stored_denoms.has(store, &to_denom) {
        return Err(OsmosisHelperError::InvalidRouteDenom {
            denom: to_denom.to_string(),
            label: "known to known to denom".to_string(),
        });
    }

    unsafe_generate_known_to_known_route(
        &pools,
        &denoms,
        from_denom,
        to_denom,
        KnownRoutePools {
            from_token_osmo_pool: stored_pools.osmo.may_load(store, &from_denom)?,
            to_token_osmo_pool: stored_pools.osmo.may_load(store, &to_denom)?,
            from_token_usdc_pool: stored_pools.usdc.may_load(store, &from_denom)?,
            to_token_usdc_pool: stored_pools.usdc.may_load(store, &to_denom)?,
        },
    )
}

/// Generates a route for a swap from a known denom to another known denom
/// given a set of related pool routes that were presumably pulled from contract state.
/// OUTPOST CODE SHOULD NOT CALL THIS FUNCTION. USE `generate_known_to_known_route` INSTEAD.
pub fn unsafe_generate_known_to_known_route(
    pools: &DestProjectSwapRoutes,
    denoms: &Denoms,
    from_denom: &str,
    to_denom: &str,
    related_pools: KnownRoutePools,
) -> Result<Vec<SwapAmountInRoute>, OsmosisHelperError> {
    match related_pools {
        // special case where we're going to or from osmo and there's an osmo pool
        KnownRoutePools {
            from_token_osmo_pool: Some(pool_id),
            ..
        }
        | KnownRoutePools {
            to_token_osmo_pool: Some(pool_id),
            ..
        } if to_denom.eq(&denoms.osmo) || from_denom.eq(&denoms.osmo) => {
            Ok(vec![SwapAmountInRoute {
                pool_id,
                token_out_denom: to_denom.to_string(),
            }])
        }
        // special case where we're going to usdc and there's a usdc pool
        KnownRoutePools {
            from_token_usdc_pool: Some(pool_id),
            ..
        }
        | KnownRoutePools {
            to_token_usdc_pool: Some(pool_id),
            ..
        } if to_denom.eq(&denoms.usdc) || from_denom.eq(&denoms.usdc) => {
            Ok(vec![SwapAmountInRoute {
                pool_id,
                token_out_denom: to_denom.to_string(),
            }])
        }
        // can swap via osmo (for example MBRN -> OSMO -> WHALE)
        KnownRoutePools {
            from_token_osmo_pool: Some(in_pool_id),
            to_token_osmo_pool: Some(out_pool_id),
            ..
        } => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.osmo.clone(),
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom.to_string(),
            },
        ]),
        // can swap via usdc (for example CDT -> USDC -> axlUSDC)
        KnownRoutePools {
            from_token_usdc_pool: Some(in_pool_id),
            to_token_usdc_pool: Some(out_pool_id),
            ..
        } => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.usdc.clone(),
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom.to_string(),
            },
        ]),

        // can swap to osmo and then to usdc and then out (for example MARS -> OSMO -> USDC -> axlUSDC)
        KnownRoutePools {
            from_token_osmo_pool: Some(in_pool_id),
            to_token_usdc_pool: Some(out_pool_id),
            ..
        } => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.osmo.clone(),
            },
            SwapAmountInRoute {
                pool_id: pools.osmo_pools.usdc.pool_id,
                token_out_denom: denoms.usdc.clone(),
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom.to_string(),
            },
        ]),

        // can swap to usdc and then to osmo and then out (for example axlUSDC -> USDC -> OSMO -> ION)
        KnownRoutePools {
            from_token_usdc_pool: Some(in_pool_id),
            to_token_osmo_pool: Some(out_pool_id),
            ..
        } => Ok(vec![
            SwapAmountInRoute {
                pool_id: in_pool_id,
                token_out_denom: denoms.usdc.clone(),
            },
            SwapAmountInRoute {
                pool_id: pools.usdc_pools.osmo.pool_id,
                token_out_denom: denoms.osmo.clone(),
            },
            SwapAmountInRoute {
                pool_id: out_pool_id,
                token_out_denom: to_denom.to_string(),
            },
        ]),
        _ => Err(OsmosisHelperError::NoKnownToKnownRoute {
            from_denom: from_denom.to_string(),
            to_denom: to_denom.to_string(),
        }),
    }
}

/// Queries the osmosis for the amount of `to_denom` that can be received for `from_token`
/// Returns both the swap simulation and the queried swap route
pub fn simulate_swap(
    querier: &QuerierWrapper,
    from_token: &CWCoin,
    route: Vec<SwapAmountInRoute>,
) -> StdResult<(EstimateSwapExactAmountInResponse, Vec<SwapAmountInRoute>)> {
    let estimate = EstimateSwapExactAmountInRequest {
        // sender: delegator_address.to_string(),
        pool_id: route.clone().first().unwrap().pool_id,
        token_in: format!("{}{}", from_token.amount, from_token.denom),
        routes: route.clone(),
    }
    .query(querier)?;

    Ok((estimate, route))
}

pub fn generate_known_to_known_swap_and_sim_msg(
    querier: &QuerierWrapper,
    store: &dyn Storage,
    twap_duration_seconds: &u64,
    pool_routes: OsmosisRoutePools,
    user_addr: &Addr,
    from_asset: &CWCoin,
    to_denom: &str,
    current_time: Timestamp,
) -> Result<(Uint128, Vec<CosmosProtoMsg>), OsmosisHelperError> {
    generate_swap_and_sim_msg(
        querier,
        twap_duration_seconds,
        user_addr,
        from_asset,
        to_denom.to_string(),
        generate_known_to_known_route(store, pool_routes, &from_asset.denom, to_denom)?,
        current_time,
    )
}

pub fn generate_known_to_unknown_swap_and_sim_msg(
    querier: &QuerierWrapper,
    store: &dyn Storage,
    twap_duration_seconds: &u64,
    pool_routes: OsmosisRoutePools,
    user_addr: &Addr,
    from_asset: &CWCoin,
    to_asset: TargetAsset,
    current_time: Timestamp,
) -> Result<(Uint128, Vec<CosmosProtoMsg>), OsmosisHelperError> {
    generate_swap_and_sim_msg(
        querier,
        twap_duration_seconds,
        user_addr,
        from_asset,
        to_asset.denom.clone(),
        generate_known_to_unknown_route(store, pool_routes, &from_asset.denom, to_asset)?,
        current_time,
    )
}

/// Generates the swap message and the simulated response given a route
pub fn generate_swap_and_sim_msg(
    querier: &QuerierWrapper,
    twap_duration_seconds: &u64,
    user_address: &Addr,
    from_asset: &CWCoin,
    to_denom: String,
    route: Vec<SwapAmountInRoute>,
    current_time: Timestamp,
) -> Result<(Uint128, Vec<CosmosProtoMsg>), OsmosisHelperError> {
    if from_asset.denom.eq(&to_denom) {
        return Ok((from_asset.amount.clone(), vec![]));
    }

    let (simulation, _routes) = simulate_swap(querier, from_asset, route.clone())?;

    let simulation = Uint128::from_str(simulation.token_out_amount.as_str())?;

    let swap_msgs = vec![generate_swap(
        from_asset,
        user_address,
        route.clone(),
        estimate_token_out_min_amount(
            querier,
            twap_duration_seconds,
            &route,
            from_asset.denom.clone(),
            from_asset.amount,
            current_time,
        )?,
    )];

    Ok((simulation, swap_msgs))
}

pub fn estimate_token_out_min_amount(
    querier: &QuerierWrapper,
    twap_duration_seconds: &u64,
    route: &Vec<SwapAmountInRoute>,
    denom_in: String,
    amount_in: Uint128,
    current_time: Timestamp,
) -> Result<Uint128, OsmosisHelperError> {
    let twap = TwapQuerier::new(querier);

    let mut in_denom = denom_in;
    let mut token_out_min_amount = amount_in;
    for route_section in route.iter() {
        let twap_start = current_time.minus_seconds(twap_duration_seconds.to_owned());
        // get the twap for this section of the multihop route
        let resp = twap.arithmetic_twap_to_now(
            route_section.pool_id,
            in_denom,
            route_section.token_out_denom.clone(),
            Some(shim::Timestamp {
                seconds: i64::try_from(twap_start.seconds())?,
                nanos: 0,
            }),
        )?;

        // update the in denom for the next section
        in_denom = route_section.token_out_denom.clone();
        // calculate the minimum amount of the next token to receive
        token_out_min_amount = amount_in * Decimal::from_str(&resp.arithmetic_twap)?;
    }

    // return 99% of the calculated amount to account for fluctuation in the past minute plus swap fees
    Ok(token_out_min_amount * Decimal::percent(99u64))
}

pub fn generate_swap(
    from_asset: &CWCoin,
    user_addr: &Addr,
    routes: Vec<SwapAmountInRoute>,
    token_out_min_amount: Uint128,
) -> CosmosProtoMsg {
    CosmosProtoMsg::OsmosisSwapExactAmountIn(MsgSwapExactAmountIn {
        token_in: Some(Coin {
            denom: from_asset.denom.to_string(),
            amount: from_asset.amount.to_string(),
        }),
        sender: user_addr.to_string(),
        routes,
        token_out_min_amount: token_out_min_amount.to_string(),
    })
}

pub fn simulate_pool_swap(
    querier: &QuerierWrapper,
    pool_id: &u64,
    offer_token: &CWCoin,
    token_out_denom: &str,
) -> Result<EstimateSwapExactAmountInResponse, OsmosisHelperError> {
    let simulation = EstimateSwapExactAmountInRequest {
        pool_id: pool_id.clone(),
        token_in: format!("{}{}", offer_token.amount, offer_token.denom),
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
    offer_asset: &CWCoin,
    token_out_denom: &str,
    token_out_min_amount: Uint128,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), OsmosisHelperError> {
    Ok((
        vec![CosmosProtoMsg::OsmosisSwapExactAmountIn(
            MsgSwapExactAmountIn {
                token_in: Some(Coin {
                    denom: offer_asset.denom.to_string(),
                    amount: offer_asset.amount.to_string(),
                }),
                sender: user_addr.to_string(),
                token_out_min_amount: token_out_min_amount.to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: *pool_id,
                    token_out_denom: token_out_denom.to_string(),
                }],
            },
        )],
        Uint128::from_str(
            simulate_pool_swap(querier, pool_id, offer_asset, token_out_denom)?
                .token_out_amount
                .as_str(),
        )?,
    ))
}

pub fn osmosis_swap_grants(base: GrantBase) -> Vec<GrantRequirement> {
    vec![GrantRequirement::generic_auth(
        base.clone(),
        MsgSwapExactAmountIn::TYPE_URL,
    )]
}
