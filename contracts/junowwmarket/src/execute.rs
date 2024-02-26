use cosmwasm_std::{Addr, Attribute, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg};
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, DestProjectMsgs, TaxSplitResult},
    msg_gen::create_exec_msg,
};
use std::iter;




use crate::{
    msg::ContractAddrs,
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError, helpers::{query_and_generate_ww_market_reward_msgs, asset_to_coin, terraswap_assetinfo_to_wyndex_assetinfo},
};
use wynd_helpers::wynd_swap::{create_wyndex_swap_msg_with_simulation, simulate_and_swap_wynd_pair, wynd_pair_swap_msg};
use wyndex::asset::{Asset, AssetInfo};
use juno_destinations::comp_prefs::{JunoCompPrefs, JunoDestinationProject, StakingDao};
use juno_destinations::dest_project_gen::{balance_dao_msgs, gelotto_lottery_msgs, mint_juno_lsd_msgs, racoon_bet_msgs, send_tokens_msgs, wynd_staking_msgs};
use sail_destinations::dest_project_gen::{spark_ibc_msgs, white_whale_satellite_msgs};
use universal_destinations::dest_project_gen::{daodao_cw20_staking_msg, native_staking_msg};

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    project_addresses: ContractAddrs,
    delegator_address: String,
    comp_prefs: JunoCompPrefs,
    tax_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let delegator: Addr = deps.api.addr_validate(&delegator_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &delegator, ADMIN, AUTHORIZED_ADDRS)?;

   let TaxSplitResult {
        remaining_rewards,
        tax_amount,
        claim_and_tax_msgs,
    } = query_and_generate_ww_market_reward_msgs(
        tax_fee.unwrap_or(Decimal::percent(5)), 
        &delegator, &project_addresses.take_rate_addr.clone(), 
        &project_addresses.destination_projects.white_whale.rewards.clone(), 
        &project_addresses.destination_projects.white_whale.market.clone(),
        &project_addresses.terraswap_routes.whale_asset.to_string(),
        &deps.querier)?;

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(
        &project_addresses,
        &delegator,
        remaining_rewards.clone(),
        comp_prefs,
        deps.as_ref(),
    )?;

    let mut combined_msgs = all_msgs.iter().fold(DestProjectMsgs::default(), |mut acc, msg| {
        acc.msgs.append(&mut msg.msgs.clone());
        acc.sub_msgs.append(&mut msg.sub_msgs.clone());
        acc.events.append(&mut msg.events.clone());
        acc
    });

    // add the claim and tax msgs to the list of msgs to be broadcast. do them first so all the funds are in place for compounding
    combined_msgs.prepend_msgs(claim_and_tax_msgs);
    combined_msgs.prepend_events(vec![Event::new("tax").add_attribute("amount", tax_amount.to_string())]);

    // amount_automated is standardized and emitted across all outposts for record keeping purposes
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
        // .add_attribute("amount_automated", to_json_binary(&[total_rewards])?.to_string())
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
    delegator_addr: &Addr,
    total_rewards: cosmwasm_std::Coin,
    comp_prefs: JunoCompPrefs,
    deps: Deps,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, JunoDestinationProject::JunoStaking { validator_address: "juno1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

    let terraswap_multihop_addr = project_addrs.destination_projects.white_whale.terraswap_multihop_router.clone();
   
    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Vec<DestProjectMsgs> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<DestProjectMsgs, ContractError> {
                

                match destination {
                    JunoDestinationProject::JunoStaking { validator_address } => {
                        let (swap_msg, simulated_juno) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                "ujuno", 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;
                        
                        let mut staking_msgs = native_staking_msg(
                            &validator_address,
                            delegator_addr,
                            &asset_to_coin(simulated_juno)?,
                        )?;

                    

                    staking_msgs.append_msgs(vec![swap_msg]);
                    
                    Ok(staking_msgs)
                },

                    JunoDestinationProject::DaoStaking(dao) => {
                        if let StakingDao::Kleomedes = dao {
                            let mut noop_resp = DestProjectMsgs::default();

                            noop_resp.events.push(Event::new("dao_stake")
                                .add_attribute("dao", dao.to_string())
                                .add_attribute("status", "disabled"));

                            return Ok(noop_resp);
                        }


                        let dao_addresses = dao.get_daos_addresses(&project_addrs.destination_projects.daos);

                        let (terraswap_swap_msg, simulated_juno) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                "ujuno", 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;

                        let (wyndex_swap_msgs, expected_dao_token_amount) = if let Some(pair_addr) = dao_addresses.juno_wyndex_pair
                        {
                            // if there's a direct juno & staking denom pair, then we can swap directly
                            let (swap_msg, swap_sim) = simulate_and_swap_wynd_pair(
                                &deps.querier,
                                delegator_addr,
                                pair_addr.as_ref(),
                                wyndex::asset::Asset {
                                    info: terraswap_assetinfo_to_wyndex_assetinfo(simulated_juno.info),
                                    amount: simulated_juno.amount,
                                },
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                            )?;

                            (vec![swap_msg], swap_sim.return_amount)
                        } else {
                            // otherwise we need to use the wyndex router to swap
                            create_wyndex_swap_msg_with_simulation(
                                &deps.querier,
                                delegator_addr,
                                comp_token_amount,
                                AssetInfo::Native("ujuno".to_string()),
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                                project_addrs.destination_projects.wynd.multihop.to_string(),
                                None
                            )?
                        };

                        let mut stake_msgs = daodao_cw20_staking_msg(
                            dao.to_string(),
                            delegator_addr,
                            &dao_addresses.cw20,
                            &dao_addresses.staking,
                            expected_dao_token_amount,
                        )?;

                        // order is important here
                        stake_msgs.prepend_msgs(wyndex_swap_msgs);
                        // we need to do the terraswap swap before the wyndex one
                        stake_msgs.prepend_msgs(vec![terraswap_swap_msg]);

                        Ok(stake_msgs)
                    }

                    JunoDestinationProject::WyndStaking { bonding_period } => {
                        let cw20 = project_addrs.destination_projects.wynd.cw20.to_string();
                        let juno_wynd_pair = project_addrs.destination_projects.wynd.juno_wynd_pair.to_string();

                        // swap uwhale for juno
                        let (juno_swap_msg, simulated_juno) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                "ujuno", 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;

                        // swap juno for wynd
                        let wynd_swap_msg = wynd_pair_swap_msg(
                            delegator_addr,
                            Asset {
                                info: AssetInfo::Native("ujuno".to_string()),
                                amount: simulated_juno.amount,
                            },
                            AssetInfo::Token(cw20.to_string()),
                            &juno_wynd_pair,
                        )?;

                        let mut staking_msg =
                            wynd_staking_msgs(&cw20, &delegator_addr.to_string(), comp_token_amount, bonding_period)?;

                        staking_msg.prepend_msgs(vec![wynd_swap_msg]);
                        staking_msg.prepend_msgs(vec![juno_swap_msg]);

                        Ok(staking_msg)
                    }

                    JunoDestinationProject::TokenSwap { target_denom } => Ok(DestProjectMsgs {
                        msgs: vec![project_addrs.terraswap_routes.gen_whale_swap(
                            delegator_addr,
                            comp_token_amount,
                            &target_denom.to_string(),
                            &terraswap_multihop_addr,
                        )?],                        
                        sub_msgs: vec![],
                        events: vec![],
                    }),
                    JunoDestinationProject::WyndLp {
                        ..
                        // contract_address,
                        // bonding_period,
                    } => Ok(DestProjectMsgs::default()),
                        JunoDestinationProject::GelottoLottery { lottery, lucky_phrase } => {
                            
                            // swap uwhale for juno
                            let (juno_swap_msg, simulated_juno) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                "ujuno", 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;

                            let mut lottery_msgs = gelotto_lottery_msgs(
                            delegator_addr,
                            project_addrs.take_rate_addr.clone(),
                            lottery,
                            &project_addrs.destination_projects.gelotto,
                            lucky_phrase,
                            simulated_juno.amount,
                        )?;

                        lottery_msgs.prepend_msgs(vec![juno_swap_msg]);

                        Ok(lottery_msgs)
                    },
                    JunoDestinationProject::RacoonBet { game } => {
                        // swap uwhale for usdc
                        let (usdc_swap_msg, simulated_usdc) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                project_addrs.usdc.to_string().as_str(), 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;
                        
                        
                        let mut game_msgs = racoon_bet_msgs(
                        &deps.querier,
                        delegator_addr,
                        // dont pass the wyndex pair in because we are passing in usdc and can skip the query
                        None,
                        asset_to_coin(simulated_usdc)?,
                        game,
                        &project_addrs.destination_projects.racoon_bet.game,
                    )?;

                    game_msgs.prepend_msgs(vec![usdc_swap_msg]);
                    
                    Ok(game_msgs)
                    },
                    JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                        // swap uwhale for the lsd via it's pair
                        let (lsd_swap_msg, simulated_asset) = 
                        project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                            comp_token_amount, 
                            asset.to_string().as_str(), 
                            &terraswap_multihop_addr, 
                            &deps.querier)?;   

                        // now just bond it to the satellite
                        let mut bond_msgs = white_whale_satellite_msgs(
                            delegator_addr,
                            asset_to_coin(simulated_asset)?,
                            &project_addrs.destination_projects.white_whale.market.clone(),
                        )?;

                        bond_msgs.prepend_msgs(vec![lsd_swap_msg]);

                        Ok(bond_msgs)
                    }
                    JunoDestinationProject::BalanceDao {} => {
                         // swap uwhale for juno
                         let (juno_swap_msg, simulated_juno) = 
                         project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                             comp_token_amount, 
                             "ujuno", 
                             &terraswap_multihop_addr, 
                             &deps.querier)?;
                        
                        let mut  balance_msgs = balance_dao_msgs(
                        delegator_addr,
                        &project_addrs.destination_projects.balance_dao,
                        simulated_juno.amount,
                    )?;

                    balance_msgs.prepend_msgs(vec![juno_swap_msg]);
                
                    Ok(balance_msgs)
                    },

                    JunoDestinationProject::MintLsd { lsd_type } => {
                       // swap uwhale for juno
                       let (juno_swap_msg, simulated_juno) = 
                       project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                           comp_token_amount, 
                           "ujuno", 
                           &terraswap_multihop_addr, 
                           &deps.querier)?;

                        let mut mint_msgs = mint_juno_lsd_msgs(
                        delegator_addr,
                        lsd_type,
                        simulated_juno.amount,
                        project_addrs.destination_projects.juno_lsds.clone(),
                    )?;

                    mint_msgs.prepend_msgs(vec![juno_swap_msg]);
                
                    Ok(mint_msgs)
                    },
                    JunoDestinationProject::SparkIbcCampaign { fund } => {
                        let spark_addr = project_addrs.destination_projects.spark_ibc.fund.clone();

                        // swap uwhale for usdc
                        let (usdc_swap_msg, simulated_usdc) = 
                            project_addrs.terraswap_routes.gen_whale_swap_with_sim(delegator_addr, 
                                comp_token_amount, 
                                project_addrs.usdc.to_string().as_str(), 
                                &terraswap_multihop_addr, 
                                &deps.querier)?;

                        let mut spark_msgs = spark_ibc_msgs(
                            delegator_addr,
                            &spark_addr,
                            asset_to_coin(simulated_usdc)?,
                            fund,
                        )?;

                        spark_msgs.prepend_msgs(vec![usdc_swap_msg]);

                        Ok(spark_msgs)
                    }
                    JunoDestinationProject::SendTokens {
                        denom: target_asset,
                        address: to_address,
                    } => {
                        // swap uwhale for the target asset
                        let (swap_msg, sim) = project_addrs.terraswap_routes.gen_whale_swap_with_sim(
                            
                            delegator_addr,
                            comp_token_amount,
                            &target_asset.to_string(),
                            &terraswap_multihop_addr,
                            &deps.querier,
                        )?;



                        // after the swap we can send the estimated funds to the target address
                        let mut send_msgs = send_tokens_msgs(
                            delegator_addr,
                            &deps.api.addr_validate(&to_address)?,
                            Asset {
                                info: target_asset,
                                amount: sim.amount,
                            },
                        )?;

                        send_msgs.append_msgs(vec![swap_msg]);

                        Ok(send_msgs)
                    }
                    JunoDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
                }
            
            },
        )
        .collect::<Result<Vec<_>, ContractError>>()?;
    // .map(|msgs_list|
    //     msgs_list.into_iter().flatten().collect());

    // withdraw_rewards_msgs.append(&mut compounding_msgs?);

    // Ok(withdraw_rewards_msgs)
    // Ok(vec![])

    Ok(compounding_msgs)
}
