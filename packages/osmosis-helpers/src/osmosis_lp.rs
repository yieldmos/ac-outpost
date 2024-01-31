use cosmwasm_std::{Addr, Coin, Event, QuerierWrapper, StdError, Uint128};
use osmosis_std::types::osmosis::gamm::v1beta1::MsgJoinSwapExternAmountIn;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as OsmosisCoin, osmosis::poolmanager::v1beta1::PoolmanagerQuerier,
};
use outpost_utils::{helpers::DestProjectMsgs, msg_gen::CosmosProtoMsg};

use crate::errors::OsmosisHelperError;

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
    lower_tick: Uint128,
    upper_tick: Uint128,
    token_min_amount_0: Uint128,
    token_min_amount_1: Uint128,
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

    // we're only going to work on pools with two assets atm
    if pool_info.pool_assets.len() != 2 {
        return Err(OsmosisHelperError::PoolHasIncorrectAssetsNum {
            pool_id,
            pool_assets_len: pool_info.pool_assets.len() as u64,
        });
    }

    return unimplemented!();
}
