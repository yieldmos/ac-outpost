use cosmwasm_std::{coin, Addr, Coin, Event, QuerierWrapper, StdError, Uint128};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::MsgCreatePosition;
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgJoinSwapExternAmountIn, PoolAsset};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin, osmosis::poolmanager::v1beta1::PoolmanagerQuerier,
};
use outpost_utils::{helpers::DestProjectMsgs, msg_gen::CosmosProtoMsg};

use crate::errors::OsmosisHelperError;
use crate::osmosis_swap::pool_swap_with_sim;

pub fn join_osmosis_pool_single_side(
    user_addr: &Addr,
    pool_id: u64,
    offer_asset: Coin,
    bond_tokens: bool,
) -> Result<Vec<CosmosProtoMsg>, OsmosisHelperError> {
    // TODO: need to consider bonding as well
    Ok(vec![CosmosProtoMsg::OsmosisSingleSidedJoinPool(
        MsgJoinSwapExternAmountIn {
            sender: user_addr.to_string(),
            pool_id,
            token_in: Some(OsmosisCoin {
                denom: offer_asset.denom.to_string(),
                amount: offer_asset.amount.to_string(),
            }),
            share_out_min_amount: "0".to_string(),
        },
    )])
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
    let pool_info: osmosis_std::types::osmosis::gamm::v1beta1::Pool = pool_querier
        .pool(pool_id)?
        .pool
        .ok_or_else(|| OsmosisHelperError::PoolNotFound { pool_id })?
        .try_into()
        .map_err(|_| {
            StdError::generic_err(format!("failed to parse pool info. pool id: {}", pool_id))
        })?;

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
        // // TODO: move this simulate to rely on a fn from osmos_swap.rs
        // let swap_est = pool_querier.estimate_swap_exact_amount_in(
        //     pool_id,
        //     coin_to_swap.to_string(),
        //     vec![SwapAmountInRoute {
        //         pool_id,
        //         token_out_denom: coin_b.denom.clone(),
        //     }],
        // )?;

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
