use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};

use crate::{
    msg::{CompoundPrefs, ExecuteMsg},
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

///
pub fn sum_coins(denom: &String, coins: &Vec<Coin>) -> Coin {
    coins
        .iter()
        .filter(|x| x.denom == denom.clone())
        .fold(Coin::new(0, denom), |a, b| {
            Coin::new((a.amount + b.amount).into(), denom)
        })
}

pub fn calculate_compound_amounts(
    percentages: &Vec<Decimal>,
    total_amount: &Uint128,
) -> Result<Vec<Uint128>, ContractError> {
    let mut remaining = total_amount.clone();
    let mut amounts = vec![];
    for (i, pct) in percentages.iter().enumerate() {
        if (i + 1) == percentages.len() {
            amounts.push(remaining);
            break;
        }
        let pct_amount = Decimal::new(total_amount.clone())
            .checked_mul(pct.clone())
            .unwrap()
            .atomics()
            .into();
        amounts.push(pct_amount);
        remaining = remaining.checked_sub(pct_amount)?;
    }

    Ok(amounts)
}

pub fn prefs_sum_to_one(comp_prefs: &CompoundPrefs) -> Result<bool, ContractError> {
    let total_pref_amounts =
        comp_prefs
            .relative
            .iter()
            .map(|x| x.amount.quantity)
            .fold(Decimal::zero(), |acc, x| {
                // need to remove this unwrap
                acc + Decimal::from_atomics(x, 18).unwrap()
            });

    Ok(total_pref_amounts == Decimal::one())
}
