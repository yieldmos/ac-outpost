use std::{iter, str::FromStr};

use cosmos_sdk_proto::cosmos::{
    base::v1beta1::Coin, distribution::v1beta1::MsgWithdrawDelegatorReward,
    staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{to_binary, Addr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128};

use osmosis_helpers::osmosis_swap::generate_swap_msg;
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one},
    msg_gen::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
    osmosis_comp_prefs::{OsmosisCompPrefs, OsmosisDestinationProject},
    queries::{query_pending_rewards, AllPendingRewards, PendingReward},
};

use crate::{
    queries::query_depositable_token_amount,
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};

pub const RED_BANK_ADDRESS: &str =
    "osmo1c3ljch9dfw5kf52nfwpxd2zmj2ese7agnx0p9tenkrryasrle5sqf3ftpg";

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegator_address: String,
    comp_prefs: OsmosisCompPrefs,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let delegator: Addr = deps.api.addr_validate(&delegator_address)?;

    // validate that the user is authorized to compound
    let _ = is_authorized_compounder(
        deps.as_ref(),
        &info.sender,
        &delegator,
        ADMIN,
        AUTHORIZED_ADDRS,
    )?;

    // get the denom of the staking token. this should be "uosmo"
    let staking_denom = deps.querier.query_bonded_denom()?;

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let sub_msgs = prefs_to_msgs(
        staking_denom.to_string(),
        &delegator,
        query_pending_rewards(&deps.querier, &delegator, staking_denom)?,
        comp_prefs,
        deps.querier,
    )?;

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, sub_msgs)?;

    Ok(Response::default().add_message(exec_msg))
}

/// Converts the user's compound preferences into a list of CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    staking_denom: String,
    delegator_address: &Addr,
    AllPendingRewards {
        rewards: pending_rewards,
        total: total_rewards,
    }: AllPendingRewards,
    comp_prefs: OsmosisCompPrefs,
    querier: QuerierWrapper,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // generate the withdraw rewards messages to grab all of the user's pending rewards
    // these should be the first msgs in the tx so the user has funds to compound
    let mut withdraw_rewards_msgs: Vec<CosmosProtoMsg> = pending_rewards
        .iter()
        .map(|PendingReward { validator, .. }| {
            CosmosProtoMsg::WithdrawDelegatorReward(MsgWithdrawDelegatorReward {
                validator_address: validator.to_string(),
                delegator_address: delegator_address.to_string(),
            })
        })
        .collect();

    // calculates the amount of uosmo that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, OsmosisDestinationProject::OsmosisStaking { validator_address: "osmo1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Result<Vec<CosmosProtoMsg>, ContractError> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<Vec<CosmosProtoMsg>, ContractError> { match destination {
                OsmosisDestinationProject::OsmosisStaking { validator_address } => {
                    Ok(vec![CosmosProtoMsg::Delegate(MsgDelegate {
                        validator_address,
                        amount: Some(Coin {
                            denom: total_rewards.denom.clone(),
                            amount: comp_token_amount.into(),
                        }),
                        delegator_address: delegator_address.to_string(),
                    })])
                },
                OsmosisDestinationProject::TokenSwap { target_denom } => {
                    let (_, swap_msg) = generate_swap_msg(&querier, delegator_address, osmosis_std::types::cosmos::base::v1beta1::Coin {
                        amount: comp_token_amount.into(),
                        denom: staking_denom.clone()
                    }, target_denom)?;

                    Ok(swap_msg)
                },
                OsmosisDestinationProject::RedBankDeposit { target_denom } =>
                    Ok(swap_and_deposit_to_redbank(
                        &querier,
                        delegator_address,
                        staking_denom.clone(),
                        target_denom,
                        comp_token_amount,
                    )?),

                // OsmosisDestinationProject::OsmosisLiquidityPool { pool_id } => {
                //     unimplemented!("liquidity pool")
                // },
                _ => unimplemented!()
            } },
        )
        .collect::<Result<Vec<_>, ContractError>>()
        .map(|msgs_list| msgs_list.into_iter().flatten().collect());

    withdraw_rewards_msgs.append(&mut compounding_msgs?);

    Ok(withdraw_rewards_msgs)
}

/// Generates the swap messages necessary to get to the denom we would like to deposit
/// into the redbank and also the actual deposit messages if deposits are not capped for that asset
/// IMPORTANT: if the deposit cap is reached, the compounding will not be forced to
/// error out. Instead, the alloted funds for depositing will remain liquid and unswapped and undeposited
fn swap_and_deposit_to_redbank(
    querier: &QuerierWrapper,
    delegator_address: &Addr,
    staking_denom: String,
    target_denom: String,
    comp_token_amount: Uint128,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // verify that the target denom is depositable
    let depositable_amount: Uint128 =
        query_depositable_token_amount(querier, target_denom.clone())?;

    if depositable_amount.is_zero() {
        // if the deposit cap is reached then there's nothing to do
        return Ok(vec![]);
    }

    // grab the msg(s) to do the swap into the target denom that we need to deposit
    let (swap_sim, mut swap_msg) = generate_swap_msg(
        querier,
        delegator_address,
        osmosis_std::types::cosmos::base::v1beta1::Coin {
            amount: comp_token_amount.into(),
            denom: staking_denom.clone(),
        },
        target_denom.clone(),
    )?;

    // if the depositable amount is less than the comp token amount, we will swap the
    // entire depositable amount
    let depositable_amount = depositable_amount.min(Uint128::from_str(&swap_sim.token_out_amount)?);

    // create the message for depositing to red bank
    let deposit_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        RED_BANK_ADDRESS.to_string(),
        delegator_address,
        &mars_red_bank_types::red_bank::ExecuteMsg::Deposit { on_behalf_of: None },
        Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
            denom: target_denom,
            amount: depositable_amount.into(),
        }]),
    )?);

    swap_msg.push(deposit_msg);

    Ok(swap_msg)
}
