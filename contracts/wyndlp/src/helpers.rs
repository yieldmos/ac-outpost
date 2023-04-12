use std::collections::HashMap;

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use outpost_utils::{
    comp_prefs::{PoolCatchAllDestinationAction, PoolCompoundPrefs},
    errors::OutpostError,
    helpers::{prefs_sum_to_one, WyndAssetLPMessages},
    msgs::{create_exec_contract_msg, CosmosProtoMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdError, StdResult, Uint128, WasmMsg};
use wyndex::{
    asset::{Asset, AssetInfo, AssetValidated},
    pair::PairInfo,
};

use crate::{msg::ExecuteMsg, ContractError};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

/// validate that the pools are unique and that the compounding prefs of said
/// pool prefs sum to 1 with no-zero quantities
pub fn valid_pool_prefs(pools: Vec<PoolCompoundPrefs>) -> Result<(), ContractError> {
    let mut unique_pools: Vec<String> = vec![];
    for PoolCompoundPrefs {
        pool_address: pool,
        comp_prefs,
    } in pools
    {
        if !unique_pools.contains(&pool) {
            unique_pools.push(pool);
        } else {
            return Err(ContractError::DuplicatePoolPrefs { pool });
        }
        let _ = prefs_sum_to_one(&comp_prefs)?;
    }
    Ok(())
}

pub fn valid_catch_all_pool_prefs(
    prefs: &Vec<PoolCatchAllDestinationAction>,
) -> Result<(), OutpostError> {
    let total_pref_amounts: Decimal =
        prefs
            .iter()
            .map(|x| x.amount)
            .fold(Ok(Decimal::zero()), |acc, x| {
                match (acc, Decimal::from_atomics(x, 18)) {
                    (Ok(acc), Ok(x)) if x.gt(&Decimal::zero()) => Ok(acc + x),
                    _ => Err(OutpostError::InvalidPrefQtys),
                }
            })?;

    match total_pref_amounts == Decimal::one() {
        true => Ok(()),
        false => Err(OutpostError::InvalidPrefQtys),
    }
}

pub struct PoolRewardsWithPrefs {
    pub pool: PairInfo,
    pub rewards: Vec<AssetValidated>,
    pub prefs: Vec<PoolCatchAllDestinationAction>,
}

/// Connects pools with pending rewards to their applicable compound prefs.
/// This will take into account if a set of catch all preferences is set or not.
/// Returns the list list of pools to perform compounding on along with their prefs.
pub fn assign_comp_prefs_to_pools(
    pending_rewards: Vec<(PairInfo, Vec<AssetValidated>)>,
    pool_prefs: Vec<PoolCompoundPrefs>,
    other_pools_prefs: &Option<Vec<PoolCatchAllDestinationAction>>,
) -> Vec<PoolRewardsWithPrefs> {
    let prefs_by_address: HashMap<String, PoolCompoundPrefs> = pool_prefs
        .into_iter()
        .map(|x| (x.pool_address.clone(), x))
        .collect();

    pending_rewards
        .iter()
        .filter_map(|(pair_info, assets)| {
            match (
                prefs_by_address.get(&pair_info.contract_addr.to_string()),
                other_pools_prefs,
            ) {
                (Some(prefs), _) => Some(PoolRewardsWithPrefs {
                    pool: pair_info.clone(),
                    rewards: assets.clone(),
                    prefs: prefs.comp_prefs.clone().into(),
                }),
                (_, Some(prefs)) => Some(PoolRewardsWithPrefs {
                    pool: pair_info.clone(),
                    rewards: assets.clone(),
                    prefs: prefs.clone(),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Calculates the amount of each asset to compound for each pool.
///
/// For example if the prefs specify that 25% of the rewards should be compounded
/// back to staking and 75% should go to a token swap while the rewards are 1000ubtc and 2000ujuno
/// the result should be [`[250ubtc, 500ujuno]`, `[750ubtc, 1500ujuno]`]
pub fn calculate_compound_amounts(
    comp_prefs: Vec<PoolCatchAllDestinationAction>,
    rewards: Vec<AssetValidated>,
) -> Result<Vec<Vec<AssetValidated>>, ContractError> {
    let mut remaining = rewards.clone();
    let mut amounts: Vec<Vec<AssetValidated>> = vec![];

    for (i, PoolCatchAllDestinationAction { amount: pct, .. }) in comp_prefs.iter().enumerate() {
        if (i + 1) == comp_prefs.len() {
            amounts.push(remaining);
            break;
        }

        amounts.push(reduce_assets_by_percentage(
            &rewards,
            &mut remaining,
            Decimal::from_atomics(pct.clone(), 18)?,
        )?);
    }

    Ok(amounts)
}

/// Reduces the amount of each asset by a percentage.
/// Returns a list of the amounts that were removed.
pub fn reduce_assets_by_percentage(
    total_assets: &Vec<AssetValidated>,
    remaining_assets: &mut Vec<AssetValidated>,
    percentage: Decimal,
) -> StdResult<Vec<AssetValidated>> {
    let mut removed_assets: Vec<AssetValidated> = vec![];

    for (i, asset) in remaining_assets.iter_mut().enumerate() {
        let amount_to_remove = total_assets[i].amount * percentage;

        asset.amount -= amount_to_remove;
        removed_assets.push(AssetValidated {
            amount: amount_to_remove,
            info: asset.info.clone(),
        });
    }

    Ok(removed_assets)
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
            msgs.extend(swap_msgs);
            assets.insert(
                target_asset_info.info.clone(),
                *assets
                    .get(&target_asset_info.info)
                    .unwrap_or(&Uint128::from(0u128))
                    + target_asset_info.amount.clone(),
            );
            (msgs, assets)
        },
    )
}

/// Constructs the messages required to join a pool from the prerequisite swaps.
/// This includes the provide increase allowances and provide liquidity messages
pub fn wynd_join_pool_msgs(
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
