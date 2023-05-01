use std::collections::HashMap;

use outpost_utils::{
    errors::OutpostError,
    helpers::prefs_sum_to_one,
    juno_comp_prefs::{PoolCatchAllDestinationAction, PoolCompoundPrefs},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, WasmMsg};
use wyndex::{asset::AssetValidated, pair::PairInfo};

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
