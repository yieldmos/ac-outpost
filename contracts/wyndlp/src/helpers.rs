use std::collections::HashMap;

use outpost_utils::{
    comp_prefs::{PoolCatchAllDestinationAction, PoolCompoundPrefs},
    errors::OutpostError,
    helpers::prefs_sum_to_one,
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

pub fn assign_comp_prefs_to_pools(
    pending_rewards: Vec<(PairInfo, Vec<AssetValidated>)>,
    pool_prefs: Vec<PoolCompoundPrefs>,
    other_pools_prefs: Option<Vec<PoolCatchAllDestinationAction>>,
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
                    prefs: prefs.comp_prefs.into(),
                }),
                (_, Some(prefs)) => Some(PoolRewardsWithPrefs {
                    pool: pair_info.clone(),
                    rewards: assets.clone(),
                    prefs,
                }),
                _ => None,
            }
        })
        .collect()
}
