use std::{iter, str::FromStr, ops::Div};

use cosmos_sdk_proto::cosmos::{
    distribution::v1beta1::MsgWithdrawDelegatorReward, staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{Addr, Decimal, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128};

use mars_red_bank_types::red_bank::Market;
use osmosis_helpers::osmosis_swap::generate_swap_msg;
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one},
    msg_gen::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
    osmosis_comp_prefs::{OsmosisCompPrefs, OsmosisDestinationProject},
    queries::{query_pending_rewards, AllPendingRewards, PendingReward},
};

use crate::{
    queries::{depositable_token_amount, query_denom_market},
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
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<Vec<CosmosProtoMsg>, ContractError> { 
                match destination {
                OsmosisDestinationProject::OsmosisStaking { validator_address } => {
                    Ok(vec![CosmosProtoMsg::Delegate(MsgDelegate {
                        validator_address,
                        amount: Some(cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
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
                    swap_and_deposit_to_redbank(
                        &querier,
                        delegator_address,
                        staking_denom.clone(),
                        target_denom,
                        comp_token_amount,
                    ),
                OsmosisDestinationProject::RedBankLeverLoop { ltv_ratio,  denom } => {
                    

                    // calculate the target ltv ratio, if the user didn't specify one then we use 50%
                    let target_ltv: Decimal = ltv_ratio
                        .map(|user_ltv| Decimal::from_atomics(user_ltv, 18))
                        .unwrap_or(Decimal::from_atomics(500_000_000_000_000_000u128, 18))?;


                    // grab the market data from red bank, this will give us most of the 
                    // general info about how red bank handles this token.
                    // if the token isn't valid for redbank then this should error out and cancel the tx
                    let denom_market = query_denom_market(&querier, denom.clone())?;

                    // check that the target ltv is within the bounds of the market
                    // if the user selected 80% ltv but the market only allows 50% then we should error out
                    if target_ltv.gt(&denom_market.max_loan_to_value) {
                        return Err(ContractError::LTVTooHigh { 
                            user_ltv: target_ltv, max_ltv: denom_market.max_loan_to_value })
                    }

                    // check the amount of token that can be deposited at this point in time
                    let redbank_denomwide_deposit_limit = depositable_token_amount(&denom_market)?;

                    // if there's no deposit availability then we need to shortcut and do nothing on this compounding
                    if redbank_denomwide_deposit_limit.is_zero() {
                        return Ok(vec![])
                    }

                    // if the user has extra borrow capacity we can use that to do our lever loop in fewer messages to save gas
                    // leaving that for a next version however
                    // let user_position: UserPositionResponse = 
                    //     querier.query_wasm_smart(RED_BANK_ADDRESS, 
                    //         &mars_red_bank_types::red_bank::QueryMsg::UserPosition { 
                    //             user: delegator_address.to_string() })?;

                    // if the user has extra denom to deposit then we can use that to do our lever loop in fewer messages to save gas
                    // we would deposit the extra denom up front and then we can borrow and leave some of that borrowed denom to make up
                    // for the extra denom we deposited at the beginning so the liquid balance denom stays unchanged
                    // leaving that for a next version however
                    // querier.query_balance(delegator_address, denom.clone());

                    // get the swap message from the user's starting denom to the denom we will be interacting with red bank with
                    let (sim, mut swap_msgs) =
                    generate_swap_msg(
                        &querier, delegator_address,
                        osmosis_std::types::cosmos::base::v1beta1::Coin {
                            amount: comp_token_amount.into(), denom: staking_denom.clone()
                        }, denom)?;

                        // get the rest of the necessary messages in order. these should be the deposit and borrow messages
                        // combine those with our swap_token message(s)
                    swap_msgs.append( &mut redbank_lever_loop_msgs(
                        delegator_address,
                        Uint128::from_str(&sim.token_out_amount)?,
                        target_ltv,
                        denom_market,
                        // user_position,
                         redbank_denomwide_deposit_limit
                    )?);

                    Ok(swap_msgs)
                }
                OsmosisDestinationProject::RedBankPayback(payback) => {
                    unimplemented!("payback")
                }
                // OsmosisDestinationProject::OsmosisLiquidityPool { pool_id } => {
                //     unimplemented!("liquidity pool")
                // },
                // _ => unimplemented!()
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
    // grab the denom "market" info from red bank so we know if depositing is even aloud
    let denom_market: Market = query_denom_market(querier, target_denom.clone())?;

    // verify that the target denom is depositable
    let depositable_amount: Uint128 = depositable_token_amount(&denom_market)?;

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

fn redbank_lever_loop_msgs(
    delegator_address: &Addr,
    initial_deposit_amount: Uint128,
    target_ltv: Decimal,
    Market { denom, max_loan_to_value, ..  }: Market,
    // user_position: UserPositionResponse,
    max_total_deposit: Uint128,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {

    let total_to_deposit: Uint128;
    let mut total_to_borrow: Uint128;

    // if the total available deposit is less than the initial deposit amount then we skip borrowing and 
    // just deposit what's available
    if max_total_deposit <= initial_deposit_amount {
       total_to_borrow = Uint128::zero();
        total_to_deposit = max_total_deposit;
    } else {
        // deposit either the protocol max allowable or (initial_deposit_amount / (1 - target_ltv))
        // whichever is less
        total_to_deposit = 
            max_total_deposit.min(Decimal::new(initial_deposit_amount)
            .div(Decimal::one() - target_ltv).atomics());
        // borrow the difference between the total to deposit and the initial deposit amount
        total_to_borrow = total_to_deposit - initial_deposit_amount;
    }

    // the list of deposit messages and borrow messages combined and ordered. this will become the final output of the function
    let mut deposit_and_borrow_msgs: Vec<CosmosProtoMsg> = vec![];

    // how much token we have liquid in the wallet to deposit at any point during the loop
    // initially this is the starting token amount but should later be updated to be the borrowed token amounts
    let mut liquid_to_deposit = initial_deposit_amount.min(max_total_deposit);
    // how much token we are allowed to borrow at any point in time. starts at zero but should be updated when a deposit occurs
    let mut available_to_borrow = Uint128::zero();
    while total_to_deposit.gt(&Uint128::zero()) {

        // create the message for depositing to red bank
        deposit_and_borrow_msgs.push( CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            RED_BANK_ADDRESS.to_string(),
            delegator_address,
            &mars_red_bank_types::red_bank::ExecuteMsg::Deposit { on_behalf_of: None },
            Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: denom.clone(),
                amount: liquid_to_deposit.into(),
            }]),
        )?));

        // now that we've deposited we can update our borrow limit
        available_to_borrow += liquid_to_deposit * max_loan_to_value;

        // we have deposited the liquid amount so we can reset it to zero
        liquid_to_deposit = Uint128::zero();
        
        if total_to_borrow.gt(&Uint128::zero()) {
            // if we have more to borrow then we can borrow the max of what we can borrow and what we need to borrow
            let borrow_amount = total_to_borrow.min(available_to_borrow);
            // create the message for borrowing from red bank
            deposit_and_borrow_msgs.push( CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                RED_BANK_ADDRESS.to_string(),
                delegator_address,
                &mars_red_bank_types::red_bank::ExecuteMsg::Borrow { 
                    denom: denom.clone(), 
                    amount: borrow_amount, 
                    recipient: None 
                },
                Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                    denom: denom.clone(),
                    amount: borrow_amount.into(),
                }]),
            )?));

            // we have borrowed what we can for now so reduce the total_to_borrow by the appropriate amount
            total_to_borrow -= borrow_amount;

            // now that we've borrowed more tokens we have more liquid in our wallet
            liquid_to_deposit += borrow_amount;
        }        
    }

    Ok(deposit_and_borrow_msgs)
}
