use std::iter;

use cosmwasm_std::{Addr, Attribute, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg};
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{
        calc_tax_split, calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, DestProjectMsgs,
        TaxSplitResult,
    },
    juno_comp_prefs::{JunoCompPrefs, JunoDestinationProject},
    msg_gen::create_exec_msg,
};
use terraswap_helpers::terraswap_swap::create_terraswap_swap_msg_with_simulation;

use juno_helpers::dest_project_gen::{
    balance_dao_msgs, daodao_cw20_staking_msg, gelotto_lottery_msgs, mint_juno_lsd_msgs, native_staking_msg,
    racoon_bet_msgs, send_tokens_msgs, spark_ibc_msgs, white_whale_satellite_msgs, wynd_staking_msgs,
};
use wynd_helpers::wynd_swap::{create_wyndex_swap_msg_with_simulation, simulate_and_swap_wynd_pair, wynd_pair_swap_msg};
use wyndex::asset::{Asset, AssetInfo};

use crate::{
    msg::{ContractAddrs, DcaPrefs},
    state::{ADMIN, AUTHORIZED_ADDRS, PROJECT_ADDRS},
    ContractError,
};

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _project_addresses: ContractAddrs,
    user_address: String,
    comp_prefs: &DcaPrefs,
    tax_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    let DcaPrefs {
        compound_token,
        compound_preferences,
    } = comp_prefs;

    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(compound_preferences)?;

    // check that the delegator address is valid
    let user_addr: Addr = deps.api.addr_validate(&user_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &user_addr, ADMIN, AUTHORIZED_ADDRS)?;

    let project_addrs = PROJECT_ADDRS.load(deps.storage)?;

    // calculate the total amount of rewards that will be compounded
    let TaxSplitResult {
        remaining_rewards,
        tax_amount,
        tax_store_msg,
    } = calc_tax_split(
        compound_token,
        tax_fee.unwrap_or(Decimal::new(1_000_000_000_000_000u128.into())),
        user_address,
        project_addrs.take_rate_addr.to_string(),
    );

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(
        &project_addrs,
        &user_addr,
        remaining_rewards.clone(),
        compound_preferences.clone(),
        deps.as_ref(),
    )?;

    let combined_msgs = all_msgs.iter().fold(
        DestProjectMsgs {
            msgs: vec![tax_store_msg],
            sub_msgs: vec![],
            events: vec![Event::new("dca_tax").add_attribute("amount", tax_amount.to_string())],
        },
        |mut acc, msg| {
            acc.msgs.append(&mut msg.msgs.clone());
            acc.sub_msgs.append(&mut msg.sub_msgs.clone());
            acc.events.append(&mut msg.events.clone());
            acc
        },
    );

    let amount_automated_event =
        Event::new("amount_automated").add_attributes([remaining_rewards].iter().enumerate().map(|(i, coin)| Attribute {
            key: format!("amount_{}", i),
            value: coin.to_string(),
        }));

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, combined_msgs.msgs)?;

    let resp = Response::default()
        .add_attribute("action", "outpost compound")
        .add_event(amount_automated_event)
        .add_message(exec_msg)
        .add_submessages(
            combined_msgs
                .sub_msgs
                .into_iter()
                .filter_map(|sub_msg| {
                    if let (Ok(exec_msg), false) = (
                        create_exec_msg(&env.contract.address, sub_msg.1.clone()),
                        sub_msg.1.is_empty(),
                    ) {
                        Some((sub_msg.0, exec_msg, sub_msg.2))
                    } else {
                        None
                    }
                })
                .map(|(id, msg, reply_on)| SubMsg {
                    msg,
                    gas_limit: None,
                    id,
                    reply_on,
                })
                .collect::<Vec<SubMsg>>(),
        )
        .add_events(combined_msgs.events);

    Ok(resp)
}

/// Converts the user's compound preferences into a list of
/// CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    project_addrs: &ContractAddrs,
    user_addr: &Addr,
    total_rewards: cosmwasm_std::Coin,
    comp_prefs: JunoCompPrefs,
    deps: Deps,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
    let dca_denom = total_rewards.denom.clone();

    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, JunoDestinationProject::JunoStaking { validator_address: "juno1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Vec<DestProjectMsgs> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<DestProjectMsgs, ContractError> {
                let compounding_asset = Asset {
                    info: AssetInfo::Native(dca_denom.clone()),
                    amount: comp_token_amount,
                };

                match destination {
                    JunoDestinationProject::JunoStaking { validator_address } => Ok(native_staking_msg(
                        &validator_address,
                        user_addr,
                        &cosmwasm_std::Coin {
                            denom: dca_denom.clone(),
                            amount: comp_token_amount,
                        },
                    )?),

                    JunoDestinationProject::DaoStaking(dao) => {
                        let dao_addresses = dao.get_daos_addresses(&project_addrs.destination_projects.daos);

                        let (swap_msgs, expected_dao_token_amount) = if let Some(pair_addr) = dao_addresses.juno_wyndex_pair
                        {
                            // if there's a direct juno & staking denom pair, then we can swap directly
                            let (swap_msg, swap_sim) = simulate_and_swap_wynd_pair(
                                &deps.querier,
                                user_addr,
                                pair_addr.as_ref(),
                                compounding_asset,
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                            )?;

                            (vec![swap_msg], swap_sim.return_amount)
                        } else {
                            // otherwise we need to use the wyndex router to swap
                            create_wyndex_swap_msg_with_simulation(
                                &deps.querier,
                                user_addr,
                                comp_token_amount,
                                AssetInfo::Native(dca_denom.clone()),
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                                project_addrs.destination_projects.wynd.multihop.to_string(),
                                None
                            )?
                        };

                        let mut stake_msgs = daodao_cw20_staking_msg(
                            dao.to_string(),
                            user_addr,
                            &dao_addresses.cw20,
                            &dao_addresses.staking,
                            expected_dao_token_amount,
                        )?;

                        stake_msgs.prepend_msgs(swap_msgs);

                        Ok(stake_msgs)
                    }

                    JunoDestinationProject::WyndStaking { bonding_period } => {
                        let cw20 = project_addrs.destination_projects.wynd.cw20.to_string();
                        let juno_wynd_pair = project_addrs.destination_projects.wynd.juno_wynd_pair.to_string();

                        // swap juno for wynd
                        let wynd_swap_msg = wynd_pair_swap_msg(
                            user_addr,
                            Asset {
                                info: AssetInfo::Native(dca_denom.clone()),
                                amount: comp_token_amount,
                            },
                            AssetInfo::Token(cw20.to_string()),
                            &juno_wynd_pair,
                        )?;

                        let mut staking_msg =
                            wynd_staking_msgs(&cw20, &user_addr.to_string(), comp_token_amount, bonding_period)?;

                        staking_msg.prepend_msgs(vec![wynd_swap_msg]);

                        Ok(staking_msg)
                    }

                    JunoDestinationProject::TokenSwap { target_denom } => Ok(DestProjectMsgs {
                        msgs: wynd_helpers::wynd_swap::create_wyndex_swap_msg(
                            user_addr,
                            comp_token_amount,
                            AssetInfo::Native(dca_denom.clone()),
                            target_denom,
                            project_addrs.destination_projects.wynd.multihop.to_string(),
                        )
                        .map_err(ContractError::Std)?,
                        sub_msgs: vec![],
                        events: vec![],
                    }),
                    JunoDestinationProject::WyndLp {
                        ..
                        // contract_address,
                        // bonding_period,
                    } => {
                      
                        Ok(DestProjectMsgs::default())
                    }
                    JunoDestinationProject::GelottoLottery { lottery, lucky_phrase } => Ok(gelotto_lottery_msgs(
                        user_addr,
                        project_addrs.take_rate_addr.clone(),
                        lottery,
                        &project_addrs.destination_projects.gelotto,
                        lucky_phrase,
                        comp_token_amount,
                    )?),
                    JunoDestinationProject::RacoonBet { game } => Ok(racoon_bet_msgs(
                        &deps.querier,
                        user_addr,
                        Some(&project_addrs.destination_projects.racoon_bet.juno_usdc_wynd_pair),
                        cosmwasm_std::Coin {
                            denom: dca_denom.clone(),
                            amount: comp_token_amount,
                        },
                        game,
                        &project_addrs.destination_projects.racoon_bet.game,
                    )?),
                    JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                        let (swap_ops, denom) = project_addrs.destination_projects.white_whale.get_juno_swap_operations(asset)?;

                        let (swap_msgs, sim) = create_terraswap_swap_msg_with_simulation(
                            &deps.querier,
                            user_addr,
                            comp_token_amount,
                            swap_ops,
                            project_addrs
                                .destination_projects
                                .white_whale
                                .terraswap_multihop_router
                                .to_string(),
                        )?;

                        let mut bond_msgs = white_whale_satellite_msgs(
                            user_addr,
                            cosmwasm_std::Coin { denom, amount: sim },
                            &project_addrs.destination_projects.white_whale.market.clone(),
                        )?;

                        bond_msgs.prepend_msgs(swap_msgs);

                        Ok(bond_msgs)
                    }
                    JunoDestinationProject::BalanceDao {} => Ok(balance_dao_msgs(
                        user_addr,
                        &project_addrs.destination_projects.balance_dao,
                        comp_token_amount,
                    )?),

                    JunoDestinationProject::MintLsd { lsd_type } => Ok(mint_juno_lsd_msgs(
                        user_addr,
                        lsd_type,
                        comp_token_amount,
                        project_addrs.destination_projects.juno_lsds.clone(),
                    )?),
                    JunoDestinationProject::SparkIbcCampaign { fund } => {
                        let spark_addr = project_addrs.destination_projects.spark_ibc.fund.clone();

                        let (swaps, est_donation) = create_wyndex_swap_msg_with_simulation(
                            &deps.querier,
                            user_addr,
                            comp_token_amount,
                            compounding_asset.info,
                            project_addrs.usdc.clone(),
                            project_addrs.destination_projects.wynd.multihop.to_string(),None
                        )?;

                        let mut spark_msgs = spark_ibc_msgs(
                            user_addr,
                            &spark_addr,
                            cosmwasm_std::Coin {
                                denom: project_addrs.usdc.to_string(),
                                amount: est_donation,
                            },
                            fund,
                        )?;

                        spark_msgs.prepend_msgs(swaps);

                        Ok(spark_msgs)
                    }
                    JunoDestinationProject::SendTokens {
                        denom: target_asset,
                        address: to_address,
                    } => {
                        let (swap_msgs, sim) = create_wyndex_swap_msg_with_simulation(
                            &deps.querier,
                            user_addr,
                            comp_token_amount,
                            AssetInfo::Native(dca_denom.clone()),
                            target_asset.clone(),
                            project_addrs.destination_projects.wynd.multihop.to_string(),None
                        )
                        .map_err(ContractError::Std)?;

                        // after the swap we can send the estimated funds to the target address
                        let mut send_msgs = send_tokens_msgs(
                            user_addr,
                            &deps.api.addr_validate(&to_address)?,
                            Asset {
                                info: target_asset,
                                amount: sim,
                            },
                        )?;

                        send_msgs.append_msgs(swap_msgs);

                        Ok(send_msgs)
                    }
                    JunoDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
                }
            
            },
        )
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(compounding_msgs)
}
