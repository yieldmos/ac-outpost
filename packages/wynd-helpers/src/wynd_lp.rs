use std::collections::HashMap;

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{StdError, Uint128};
use outpost_utils::msg_gen::{create_exec_contract_msg, CosmosProtoMsg};
use wyndex::asset::{Asset, AssetInfo};

/// Describes the prerequisite information needed when preparing to join a pool
pub struct WyndAssetLPMessages {
    /// The msgs to perform the token swaps and if applicable the increase allowances
    pub swap_msgs: Vec<CosmosProtoMsg>,
    /// The asset denom and amount that will be sent to the pool contract
    pub target_asset_info: Asset,
}

/// Combines a vector of lp messages into a vector of the underlying swap messages
/// and a hashmap of the target assets and their amounts. This is particularly suitable for preparing
/// for joining a pool.
pub fn fold_wynd_swap_msgs(
    swap_msgs: Vec<WyndAssetLPMessages>,
) -> (Vec<CosmosProtoMsg>, HashMap<AssetInfo, Uint128>) {
    swap_msgs.into_iter().fold(
        (vec![], HashMap::new()),
        |(mut msgs, mut assets),
         WyndAssetLPMessages {
             swap_msgs,
             target_asset_info,
         }| {
            // add the swap msgs to the list of msgs
            msgs.extend(swap_msgs);
            // add the target asset to the map of assets
            assets.insert(
                target_asset_info.info.clone(),
                // sum the amounts
                *assets
                    .get(&target_asset_info.info)
                    .unwrap_or(&Uint128::from(0u128))
                    + target_asset_info.amount,
            );
            (msgs, assets)
        },
    )
}

/// Constructs the messages required to join a pool from the prerequisite swaps.
/// This includes the provide increase allowances and provide liquidity messages
pub fn wynd_join_pool_from_map_msgs(
    current_block_height: &u64,
    delegator_address: String,
    pool_to_join_address: String,
    swap_msgs: &mut Vec<CosmosProtoMsg>,
    assets: HashMap<AssetInfo, Uint128>,
) -> Result<Vec<CosmosProtoMsg>, StdError> {
    let (mut native_funds, token_transfer_msgs, mut asset_funds): (
        Vec<Coin>,
        Vec<Result<CosmosProtoMsg, StdError>>,
        Vec<Asset>,
    ) = assets.into_iter().fold(
        (vec![], vec![], vec![]),
        |(mut all_native_tokens, mut all_token_transfer_msgs, mut assets), (asset, amount)| {
            match asset {
                // for native tokens we just add them to the list of native funds
                AssetInfo::Native(ref denom) => all_native_tokens.push(Coin {
                    denom: denom.clone(),
                    amount: amount.to_string(),
                }),
                // for tokens we create a cw20 increase allowance message
                AssetInfo::Token(ref token_contract_address) => all_token_transfer_msgs.push(
                    if let Ok(exec) = create_exec_contract_msg(
                        token_contract_address.clone(),
                        &delegator_address,
                        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                            spender: pool_to_join_address.to_string(),
                            amount,
                            expires: Some(cw20::Expiration::AtHeight(current_block_height + 1)),
                        },
                        None,
                    ) {
                        Ok(CosmosProtoMsg::ExecuteContract(exec))
                    } else {
                        Err(StdError::GenericErr {
                            msg: "failed to create wynd cw20 join pool message".to_string(),
                        })
                    },
                ),
            }
            // regardless if it's native or token we add it to the list of assets that
            // is needed for the provide liquidity message
            assets.push(Asset {
                info: asset,
                amount,
            });
            (all_native_tokens, all_token_transfer_msgs, assets)
        },
    );

    // Sort the assets and native tokens by their names/addresses
    asset_funds.sort_by_key(|Asset { info, .. }| match info {
        AssetInfo::Native(denom) => denom.clone(),
        AssetInfo::Token(contract_addr) => contract_addr.clone(),
    });

    native_funds.sort_by_key(|Coin { denom, .. }| denom.clone());

    // Add the cw20 transfer messages to the swap messages to prepare for joining the pool
    swap_msgs.extend(
        token_transfer_msgs
            .into_iter()
            .collect::<Result<Vec<CosmosProtoMsg>, StdError>>()?,
    );

    // Add the provide liquidity message
    swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        pool_to_join_address.clone(),
        &delegator_address,
        &wyndex::pair::ExecuteMsg::ProvideLiquidity {
            assets: asset_funds,
            slippage_tolerance: None,
            receiver: None,
        },
        Some(native_funds),
    )?));

    Ok(swap_msgs.to_vec())
}

/// Constructs the Provide Liquidity message required to join a pool from the prerequisite swaps.
/// This includes the provide increase allowances and provide liquidity messages
pub fn wynd_join_pool_msgs(
    delegator_address: String,
    pool_to_join_address: String,
    lp_asset_msgs: Vec<WyndAssetLPMessages>,
) -> Result<Vec<CosmosProtoMsg>, StdError> {
    // creates the vec of assets that go into the provide liquidity message
    let assets: Vec<Asset> = lp_asset_msgs
        .iter()
        .map(
            |WyndAssetLPMessages {
                 target_asset_info, ..
             }| target_asset_info.clone(),
        )
        .collect::<Vec<_>>();

    // the native funds that are passed into the tx
    let native_funds: Vec<Coin> = lp_asset_msgs
        .iter()
        .filter_map(
            |WyndAssetLPMessages {
                 target_asset_info, ..
             }| {
                if let Asset {
                    info: AssetInfo::Native(native_denom),
                    amount,
                } = target_asset_info
                {
                    Some(Coin {
                        denom: native_denom.clone(),
                        amount: amount.to_string(),
                    })
                } else {
                    None
                }
            },
        )
        .collect::<Vec<_>>();

    // the accumulated list of all the swap messages from the  WyndAssetLPMessages
    let mut swap_msgs: Vec<CosmosProtoMsg> = lp_asset_msgs
        .iter()
        .flat_map(|WyndAssetLPMessages { swap_msgs, .. }| swap_msgs.clone())
        .collect::<Vec<_>>();

    swap_msgs.append(&mut vec![CosmosProtoMsg::ExecuteContract(
        create_exec_contract_msg(
            pool_to_join_address,
            &delegator_address,
            &wyndex::pair::ExecuteMsg::ProvideLiquidity {
                assets,
                slippage_tolerance: None,
                receiver: None,
            },
            Some(native_funds),
        )?,
    )]);

    Ok(swap_msgs)
}
