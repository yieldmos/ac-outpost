use cosmwasm_std::{Addr, Coin, Decimal, Deps, Uint128};
use cw_storage_plus::Item;

use crate::{
    comp_prefs::{CompoundPrefs, DestinationAction},
    errors::OutpostError,
};

/// sums the coins in a vec given denom name youre looking for
pub fn sum_coins(denom: &String, coins: &[Coin]) -> Coin {
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
) -> Result<Vec<Uint128>, OutpostError> {
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
pub fn prefs_sum_to_one<D>(comp_prefs: &CompoundPrefs<D>) -> Result<bool, OutpostError> {
    let total_pref_amounts =
        comp_prefs
            .relative
            .iter()
            .map(|x| x.amount)
            .fold(Ok(Decimal::zero()), |acc, x| {
                match (acc, Decimal::from_atomics(x, 18)) {
                    (Ok(acc), Ok(x)) if x.gt(&Decimal::zero()) => Ok(acc + x),
                    _ => Err(OutpostError::InvalidPrefQtys),
                }
            })?;

    match total_pref_amounts == Decimal::one() {
        true => Ok(true),
        false => Err(OutpostError::InvalidPrefQtys),
    }
}

/// try from to a vector of decimals will give the relative percentages
/// that should be used for compounding the rewards
impl<D> TryFrom<CompoundPrefs<D>> for Vec<Decimal> {
    type Error = OutpostError;

    fn try_from(prefs: CompoundPrefs<D>) -> Result<Self, OutpostError> {
        prefs
            .relative
            .iter()
            .map(|DestinationAction { amount, .. }| {
                Decimal::from_atomics(*amount, 18).map_err(|_| OutpostError::InvalidPrefQtys)
            })
            .collect::<Result<Vec<Decimal>, OutpostError>>()
    }
}

/// The sender is only authorized if they are the admin or if they are in the
/// list of authorized compounders or if they are the delegator themselves.
pub fn is_authorized_compounder(
    deps: Deps,
    sender: &Addr,
    delegator: &Addr,
    admin: Item<Addr>,
    authorized_addrs: Item<Vec<Addr>>,
) -> Result<(), OutpostError> {
    if sender.ne(delegator) {
        if sender.ne(&admin.load(deps.storage)?) {
            if !authorized_addrs.load(deps.storage)?.contains(sender) {
                return Err(OutpostError::UnauthorizedCompounder(sender.to_string()));
            }
        }
    }
    Ok(())
}
