use cosmwasm_std::{coin, Addr, Coin, Event, QuerierWrapper, StdError, Storage, Uint128};
use cw_grant_spec::grants::{GrantBase, GrantRequirement};


use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::MsgCreatePosition;
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgJoinSwapExternAmountIn};
use osmosis_std::types::osmosis::lockup::MsgLockTokens;

use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin, osmosis::poolmanager::v1beta1::PoolmanagerQuerier,
};
use outpost_utils::{helpers::DestProjectMsgs, msg_gen::CosmosProtoMsg};

use crate::errors::OsmosisHelperError;
use crate::osmosis_swap::{
    generate_known_to_known_swap_and_sim_msg,
    osmosis_swap_grants, pool_swap_with_sim, OsmosisRoutePools,
};

pub fn query_pool_info(
    pool_querier: PoolmanagerQuerier<'_, cosmwasm_std::Empty>,
    pool_id: u64,
) -> Result<Pool, OsmosisHelperError> {
    // query the info for the pool we're trying to enter
    let pool_info: Pool = pool_querier
        .pool(pool_id)?
        .pool
        .ok_or_else(|| OsmosisHelperError::PoolNotFound { pool_id })?
        .try_into()
        .map_err(|_| {
            StdError::generic_err(format!("failed to parse pool info. pool id: {}", pool_id))
        })?;

    Ok(pool_info)
}

pub struct SingleSidedJoinSwap {
    pub join_asset: Coin,
    pub swap_msgs: Vec<CosmosProtoMsg>,
}

/// Generates the necessary swap to join the non-cl pool single sided
pub fn classic_pool_join_single_side_prepratory_swap(
    querier: &QuerierWrapper,
    store: &dyn Storage,
    user_addr: &Addr,
    pool_id: u64,
    offer_asset: &Coin,
    pool_routes: OsmosisRoutePools,
) -> Result<SingleSidedJoinSwap, OsmosisHelperError> {
    let pool_tokens = query_pool_info(PoolmanagerQuerier::new(querier), pool_id)?
        .pool_assets
        .iter()
        .map(|ass| ass.token.clone())
        .collect::<Option<Vec<_>>>()
        .ok_or(OsmosisHelperError::InvalidPoolAssetCoins)?;

    // the asset is in the pool so we dont need a swap and we
    //can directly put the offer asset into the pool
    if pool_tokens
        .iter()
        .any(|coin| offer_asset.denom.eq(&coin.denom))
    {
        return Ok(SingleSidedJoinSwap {
            join_asset: offer_asset.clone(),
            swap_msgs: vec![],
        });
    }

    // if both the offer asset and one of the pool tokens are known assets we can still do the swap
    if pool_routes.stored_pools.osmo.has(store, &offer_asset.denom)
        || pool_routes.stored_pools.usdc.has(store, &offer_asset.denom)
    {
        if let Some(target_pool_target) = pool_tokens.iter().find(|coin| {
            pool_routes.stored_pools.osmo.has(store, &coin.denom)
                || pool_routes.stored_pools.usdc.has(store, &coin.denom)
        }) {
            let (sim, swap_msgs) = generate_known_to_known_swap_and_sim_msg(
                querier,
                store,
                pool_routes,
                user_addr,
                offer_asset,
                &target_pool_target.denom,
            )?;
            return Ok(SingleSidedJoinSwap {
                join_asset: coin(sim.u128(), target_pool_target.denom.clone()),
                swap_msgs,
            });
        }
    }

    Err(OsmosisHelperError::InvalidAssets)
}

pub fn join_osmosis_pool_single_side(
    user_addr: &Addr,
    pool_id: u64,
    token_in: Coin,
    _bond_tokens: bool,
) -> Result<Vec<CosmosProtoMsg>, OsmosisHelperError> {
    let join_pool_msgs = vec![CosmosProtoMsg::OsmosisSingleSidedJoinPool(
        MsgJoinSwapExternAmountIn {
            sender: user_addr.to_string(),
            pool_id,
            token_in: Some(OsmosisCoin {
                denom: token_in.denom.to_string(),
                amount: token_in.amount.to_string(),
            }),
            share_out_min_amount: "0".to_string(),
        },
    )];

    // if bond_tokens {
    //     // TODO: this needs to be done in a submessage after the join pool
    //     // join_pool_msgs.push(CosmosProtoMsg::OsmosisLockTokens(MsgLockTokens {
    //     //     owner: user_addr.to_string(),
    //     //     duration: Some(Duration {
    //     //         seconds: 1209600,
    //     //         nanos: 1209600000000000
    //     //     }),
    //     //     coins: todo!("coin representing the gamm tokens for pool `pool_id`"),

    //     // }) );
    // }

    Ok(join_pool_msgs)
}

pub fn join_osmosis_cl_pool_single_side(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    pool_id: u64,
    offer_asset: Coin,
    lower_tick: i64,
    upper_tick: i64,
    token_min_amount0: Uint128,
    token_min_amount1: Uint128,
    // alternative_denoms
) -> Result<Vec<CosmosProtoMsg>, OsmosisHelperError> {
    let pool_querier = PoolmanagerQuerier::new(querier);

    // query the info for the pool we're trying to enter
    let pool_info = query_pool_info(pool_querier, pool_id)?;

    let pool_coins = pool_info
        .pool_assets
        .iter()
        .map(|ass| ass.token.clone())
        .collect::<Option<Vec<_>>>()
        .ok_or(OsmosisHelperError::InvalidPoolAssetCoins)?;

    // we're only going to work on pools with two assets atm
    if pool_coins.len() != 2 {
        return Err(OsmosisHelperError::PoolHasIncorrectAssetsNum {
            pool_id,
            pool_assets_len: pool_info.pool_assets.len() as u64,
        });
    }

    // check to see if the pool has the asset we're trying to enter with
    // if it does then let's order them so that the first is the asset we have
    let pool_assets = match pool_coins.as_slice() {
        [a, b] if offer_asset.denom.eq(&a.denom) => Some((a, b)),
        [a, b] if offer_asset.denom.eq(&b.denom) => Some((b, a)),
        _ => None,
    };

    // the offer asset exists in the pool and is coin_a
    if let Some((_, coin_b)) = pool_assets {
        let (mut pre_swap, est_token_b) = pool_swap_with_sim(
            querier,
            user_addr,
            &pool_id,
            coin(
                offer_asset.amount.checked_div(2u128.into())?.u128(),
                offer_asset.denom.clone(),
            ),
            &coin_b.denom,
        )?;

        pre_swap.push(CosmosProtoMsg::OsomsisCLJoinPool(MsgCreatePosition {
            pool_id,
            sender: user_addr.to_string(),
            lower_tick,
            upper_tick,
            tokens_provided: vec![
                OsmosisCoin {
                    amount: (offer_asset.amount - offer_asset.amount.checked_div(2u128.into())?)
                        .to_string(),
                    denom: offer_asset.denom.clone(),
                },
                OsmosisCoin {
                    amount: est_token_b.to_string(),
                    denom: coin_b.denom.clone(),
                },
            ],
            token_min_amount0: token_min_amount0.to_string(),
            token_min_amount1: token_min_amount1.to_string(),
        }));

        Ok(pre_swap)
    }
    // the offer asset isn't part of the pool.
    else {
        Err(OsmosisHelperError::InvalidAssets)
        // unimplemented!("check to see if we can do a quick swap to osmo or usdc and keep going")
    }
}

pub fn gen_join_cl_pool_single_sided_msgs(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    pool_id: u64,
    offer_token: &Coin,
    lower_tick: i64,
    upper_tick: i64,
    token_min_amount0: Uint128,
    token_min_amount1: Uint128,
) -> Result<DestProjectMsgs, OsmosisHelperError> {
    let join_pool_msgs = join_osmosis_cl_pool_single_side(
        querier,
        user_addr,
        pool_id,
        offer_token.clone(),
        lower_tick,
        upper_tick,
        token_min_amount0,
        token_min_amount1,
    )?;

    Ok(DestProjectMsgs {
        msgs: join_pool_msgs,
        sub_msgs: vec![],
        events: vec![Event::new("osmosis_liquidity_pool")
            .add_attribute("pool_style", "concentrated")
            .add_attribute("pool_id", pool_id.to_string())],
    })
}

pub fn gen_join_classic_pool_single_sided_msgs(
    querier: &QuerierWrapper,
    store: &dyn Storage,
    route_pools: OsmosisRoutePools,
    user_addr: &Addr,
    pool_id: u64,
    offer_token: &Coin,
    bond_tokens: bool,
) -> Result<DestProjectMsgs, OsmosisHelperError> {
    let SingleSidedJoinSwap {
        join_asset,
        swap_msgs,
    } = classic_pool_join_single_side_prepratory_swap(
        querier,
        store,
        user_addr,
        pool_id,
        offer_token,
        route_pools,
    )?;

    let join_pool_msgs =
        join_osmosis_pool_single_side(user_addr, pool_id, join_asset, bond_tokens)?;

    Ok(DestProjectMsgs {
        msgs: [swap_msgs, join_pool_msgs].concat(),
        sub_msgs: vec![],
        events: vec![Event::new("osmosis_liquidity_pool")
            .add_attribute("pool_style", "classic")
            .add_attribute("pool_id", pool_id.to_string())
            .add_attribute("bond_tokens", bond_tokens.to_string())],
    })
}

pub fn join_classic_pool_grants(base: GrantBase, bond_tokens: bool) -> Vec<GrantRequirement> {
    vec![
        // TODO we shouldn't always need the swap permission
        osmosis_swap_grants(base.clone()),
        vec![GrantRequirement::generic_auth(
            base.clone(),
            MsgJoinSwapExternAmountIn::TYPE_URL,
        )],
        if bond_tokens {
            vec![GrantRequirement::generic_auth(
                base,
                MsgLockTokens::TYPE_URL,
            )]
        } else {
            vec![]
        },
    ]
    .concat()
}

pub fn join_cl_pool_grants(base: GrantBase) -> Vec<GrantRequirement> {
    vec![
        // swap permission so we can get into the pool from a single asset
        osmosis_swap_grants(base.clone()),
        vec![GrantRequirement::generic_auth(
            base,
            MsgCreatePosition::TYPE_URL,
        )],
    ]
    .concat()
}
