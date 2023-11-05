use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Deps, Timestamp, Uint128};
use cw_storage_plus::Item;

use crate::{
    comp_prefs::{CompoundPrefs, DestinationAction},
    errors::OutpostError,
    msg_gen::CosmosProtoMsg,
};

#[cw_serde]
#[derive(Default)]
pub enum CompoundingFrequency {
    Hourly = 3600,
    TwiceDaily = 43200,
    #[default]
    Daily = 86400,
    Weekly = 604800,
    Monthly = 2592000,
    Quarterly = 7776000,
}

impl CompoundingFrequency {
    pub fn iteration_count(&self, current_time: Timestamp, end_timestamp: Timestamp) -> u64 {
        (end_timestamp.seconds() - current_time.seconds()) / (self.clone() as u64)
    }
}

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
            .try_fold(Decimal::zero(), |acc, x| {
                match (acc, Decimal::from_atomics(x, 18)) {
                    (acc, Ok(x)) if x.gt(&Decimal::zero()) => Ok(acc + x),
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
    if sender.ne(delegator)
        && sender.ne(&admin
            .load(deps.storage)
            .map_err(|_| OutpostError::AdminLoadFailure())?)
        && !authorized_addrs
            .load(deps.storage)
            .map_err(|_| OutpostError::AuthorizedAdminLoadFailure())?
            .contains(sender)
    {
        return Err(OutpostError::UnauthorizedCompounder(sender.to_string()));
    }
    Ok(())
}

#[derive(Debug, PartialEq, Clone)]
pub struct TaxSplitResult {
    pub remaining_rewards: Coin,
    pub tax_amount: Coin,
    pub tax_store_msg: CosmosProtoMsg,
}

pub fn calc_tax_split(
    token: &Coin,
    tax: Decimal,
    sender: String,
    tax_addr: String,
) -> TaxSplitResult {
    let tax_amount = token.amount.mul_ceil(tax);
    let remaining_rewards = token.amount.saturating_sub(tax_amount);

    let tax_store_msg = CosmosProtoMsg::Send(MsgSend {
        from_address: sender.to_string(),
        to_address: tax_addr.to_string(),
        amount: vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
            denom: token.denom.clone(),
            amount: tax_amount.to_string(),
        }],
    });

    TaxSplitResult {
        remaining_rewards: Coin {
            denom: token.denom.clone(),
            amount: remaining_rewards,
        },
        tax_amount: Coin {
            denom: token.denom.clone(),
            amount: tax_amount,
        },
        tax_store_msg,
    }
}
