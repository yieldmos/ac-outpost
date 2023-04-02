use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};

use crate::{
    msg::{CompoundPrefs, DestinationAction, ExecuteMsg, RelativeQty},
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

/// sums the coins in a vec given denom name youre looking for
pub fn sum_coins(denom: &String, coins: &Vec<Coin>) -> Coin {
    coins
        .iter()
        .filter(|x| x.denom == denom.clone())
        .fold(Coin::new(0, denom), |a, b| {
            Coin::new((a.amount + b.amount).into(), denom)
        })
}

/// calculates the amounts that should be sent to each destination project
pub fn calculate_compound_amounts(
    percentages: &Vec<Decimal>,
    total_amount: &Uint128,
) -> Result<Vec<Uint128>, ContractError> {
    let mut remaining = *total_amount;
    let mut amounts = vec![];
    for (i, pct) in percentages.iter().enumerate() {
        if (i + 1) == percentages.len() {
            amounts.push(remaining);
            break;
        }
        let pct_amount = Decimal::new(*total_amount).checked_mul(*pct)?.atomics();
        amounts.push(pct_amount);
        remaining = remaining.checked_sub(pct_amount)?;
    }

    Ok(amounts)
}

/// checks that the prefs are both summing to 1 and that they are all positive and nonzero
pub fn prefs_sum_to_one(comp_prefs: &CompoundPrefs) -> Result<bool, ContractError> {
    let total_pref_amounts = comp_prefs.relative.iter().map(|x| x.amount.quantity).fold(
        Ok(Decimal::zero()),
        |acc, x| match (acc, Decimal::from_atomics(x, 18)) {
            (Ok(acc), Ok(x)) if x.gt(&Decimal::zero()) => Ok(acc + x),
            _ => Err(ContractError::InvalidPrefQtys),
        },
    )?;

    Ok(total_pref_amounts == Decimal::one())
}

/// try from to a vector of decimals will give the relative percentages
/// that should be used for compounding the rewards
impl TryFrom<CompoundPrefs> for Vec<Decimal> {
    type Error = ContractError;

    fn try_from(prefs: CompoundPrefs) -> Result<Self, ContractError> {
        prefs
            .relative
            .iter()
            .map(
                |DestinationAction {
                     amount: RelativeQty { quantity },
                     ..
                 }| {
                    Decimal::from_atomics(*quantity, 18).map_err(|_| ContractError::InvalidPrefQtys)
                },
            )
            .collect::<Result<Vec<Decimal>, ContractError>>()
    }
}
