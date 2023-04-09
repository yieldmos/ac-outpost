use outpost_utils::helpers::prefs_sum_to_one;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, WasmMsg};

use crate::{
    msg::{ExecuteMsg, PoolCompoundPrefs},
    ContractError,
};

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
