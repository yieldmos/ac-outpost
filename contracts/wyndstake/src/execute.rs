use std::iter;

use cosmwasm_std::{Addr, Attribute, Coin, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg, Uint128};
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, DestProjectMsgs, RewardSplit},
    msg_gen::create_exec_msg,
};
use terraswap_helpers::terraswap_swap::create_terraswap_swap_msg_with_simulation;
use wynd_helpers::wynd_swap::simulate_and_swap_wynd_pair;
use wyndex::asset::{Asset, AssetInfo};
use juno_destinations::comp_prefs::{JunoCompPrefs, JunoDestinationProject, StakingDao};
use juno_destinations::dest_project_gen::{balance_dao_msgs, gelotto_lottery_msgs, mint_juno_lsd_msgs, racoon_bet_msgs, send_tokens_msgs, wynd_staking_msgs};
use sail_destinations::dest_project_gen::{spark_ibc_msgs, white_whale_satellite_msgs};
use universal_destinations::dest_project_gen::{daodao_cw20_staking_msg, native_staking_msg};

use crate::{
    helpers::{query_and_generate_wynd_reward_msgs, wynd_wyndex_multihop_swap, },
    msg::ContractAddrs,
    state::{ADMIN, AUTHORIZED_ADDRS, PROJECT_ADDRS},
    ContractError,
};

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _project_addresses: ContractAddrs,
    user_address: String,
    comp_prefs: &JunoCompPrefs,
    tax_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(comp_prefs)?;

    // check that the delegator address is valid
    let user_addr: Addr = deps.api.addr_validate(&user_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &user_addr, ADMIN, AUTHORIZED_ADDRS)?;

    let project_addrs = PROJECT_ADDRS.load(deps.storage)?;

    // calculate the total amount of rewards that will be compounded
    let RewardSplit {
        user_rewards,
        tax_amount,
        claim_msgs,
    } = query_and_generate_wynd_reward_msgs(
        tax_fee.unwrap_or(Decimal::percent(5)),
        &user_addr,
        &project_addrs.take_rate_addr,
        &project_addrs.wynd_stake_addr,
        &project_addrs.destination_projects.wynd.cw20,
        &deps.querier,
    )?;

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(&project_addrs, &user_addr, user_rewards, comp_prefs.clone(), deps.as_ref())?;

    let combined_msgs = all_msgs.iter().fold(
        DestProjectMsgs {
            msgs: claim_msgs,
            sub_msgs: vec![],
            events: vec![Event::new("wyndstake_tax").add_attribute("amount", format!("{}{}", tax_amount, "uwynd"))],
        },
        |mut acc, msg| {
            acc.msgs.append(&mut msg.msgs.clone());
            acc.sub_msgs.append(&mut msg.sub_msgs.clone());
            acc.events.append(&mut msg.events.clone());
            acc
        },
    );

    let amount_automated_event = Event::new("amount_automated").add_attributes(
        [Coin {
            amount: user_rewards,
            denom: "uwynd".to_string(),
        }]
        .iter()
        .enumerate()
        .map(|(i, coin)| Attribute {
            key: format!("amount_{}", i),
            value: coin.to_string(),
        }),
    );

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
                    if let (Ok(exec_msg), false) = (create_exec_msg(&env.contract.address, sub_msg.1.clone()), sub_msg.1.is_empty()) {
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
    total_rewards: Uint128,
    comp_prefs: JunoCompPrefs,
    deps: Deps,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
    let wynd_addr = project_addrs.destination_projects.wynd.cw20.clone();
    let wynd_asset_info = AssetInfo::Token(wynd_addr.to_string());
    let juno_asset_info = AssetInfo::Native("ujuno".to_string());

    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, JunoDestinationProject::JunoStaking { validator_address: "juno1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards)?,
        comp_prefs.relative,
    );

    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Vec<DestProjectMsgs> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<DestProjectMsgs, ContractError> {
                let compounding_asset = Asset {
                    info: AssetInfo::Token(wynd_addr.to_string()),
                    amount: comp_token_amount,
                };

                match destination {
                    JunoDestinationProject::JunoStaking { validator_address } =>{

                        let (swap_msg, wyndex::pair::SimulationResponse {
                            return_amount: expected_juno, ..}) = 
                            simulate_and_swap_wynd_pair(&deps.querier,
                        user_addr, 
                        project_addrs.destination_projects.wynd.juno_wynd_pair.as_ref(), 
                        compounding_asset, juno_asset_info.clone())?;

                      let mut staking_msgs =  native_staking_msg(
                        &validator_address,
                        user_addr,
                        &cosmwasm_std::Coin {
                            denom: "ujuno".to_string(),
                            amount: expected_juno,
                        },
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

                        let (swap_msgs, expected_dao_token_amount) =
                            // we can just convert the wynd to the dao token via a multihop trade
                            // there are some wynd/dao pairs that are available but have tiny tvl so multihop is going to have more reliability
                            wynd_wyndex_multihop_swap( &deps.querier,
                                user_addr,
                                comp_token_amount,
                                wynd_asset_info.clone(),
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                                project_addrs.destination_projects.wynd.multihop.to_string(),)?;
                            
                        

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
                        Ok(wynd_staking_msgs(&project_addrs.destination_projects.wynd.cw20.to_string(),
                                &user_addr.to_string(), 
                                comp_token_amount, bonding_period)?)
                    }

                    JunoDestinationProject::TokenSwap { target_denom } => {
                        let (swaps, sim) = match target_denom.clone() {
                            AssetInfo::Native(juno_denom) if juno_denom.eq("ujuno") => {
                                // if we're swapping to juno we can avoid the multihop and go straight to the pair
                                let (swap, sim) = simulate_and_swap_wynd_pair(
                                    &deps.querier, user_addr
                                    , project_addrs.destination_projects.wynd.juno_wynd_pair.as_ref(), 
                                    Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, juno_asset_info.clone())?;

                                (vec![swap], sim.return_amount)
                            }
                            _ => wynd_wyndex_multihop_swap(&deps.querier,                                
                                 user_addr, comp_token_amount,
                                  wynd_asset_info.clone(),
                                   target_denom.clone(), 
                                   project_addrs.destination_projects.wynd.multihop.to_string())?
                        };
                    
                    
                    Ok(DestProjectMsgs {
                        msgs: swaps,
                        sub_msgs: vec![],
                        events: vec![Event::new("swap").add_attribute("denom", target_denom.to_string()).add_attribute("amount", sim.to_string())],
                    })},
                    JunoDestinationProject::WyndLp {
                        ..
                        // contract_address,
                        // bonding_period,
                    } => {
                      
                        Ok(DestProjectMsgs::default())
                    }
                    JunoDestinationProject::GelottoLottery { lottery, lucky_phrase } =>{ 
                        let (juno_swap, juno_sim) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.juno_wynd_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, juno_asset_info.clone())?;

                        let mut lottery_msgs = gelotto_lottery_msgs(
                        user_addr,
                        project_addrs.take_rate_addr.clone(),
                        lottery,
                        &project_addrs.destination_projects.gelotto,
                        lucky_phrase,
                        juno_sim.return_amount,
                    )?;

                    lottery_msgs.append_msgs(vec![juno_swap]);                
                
                Ok(lottery_msgs)},
                    JunoDestinationProject::RacoonBet { game } => {
                        // if we swap straight to usdc then we already have our price estimate ready
                        let (swap, usdc_sim) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.wynd_usdc_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, project_addrs.usdc.clone())?;
                            
                       let mut rac_msgs =  racoon_bet_msgs(
                        &deps.querier,
                        user_addr,
                        // dont give the pair addr because we're pre-validating
                        None,
                        cosmwasm_std::Coin {
                            denom: project_addrs.usdc.to_string(),
                            amount: usdc_sim.return_amount,
                        },
                        game,
                        &project_addrs.destination_projects.racoon_bet.game,
                    )?;

                    rac_msgs.append_msgs(vec![swap]);

                    Ok(rac_msgs)
                },
                    JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                        // if we swap our wynd to usdc we have a shorter in wyndex we'll have a shorter path in terraswap to get the lsds
                        let (usdc_swap,  wyndex::pair::SimulationResponse {return_amount: est_usdc,..}) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.wynd_usdc_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, project_addrs.usdc.clone())?;
                            
                        let (swap_ops, denom) = project_addrs.destination_projects.white_whale.get_usdc_swap_operations(asset)?;

                        let (lsd_swap_msgs, sim) = create_terraswap_swap_msg_with_simulation(
                            &deps.querier,
                            user_addr,
                            est_usdc,
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

                        bond_msgs.prepend_msgs(lsd_swap_msgs);

                        bond_msgs.append_msgs(vec![usdc_swap]);

                        Ok(bond_msgs)
                    }
                    JunoDestinationProject::BalanceDao {} => {
                        let (juno_swap, wyndex::pair::SimulationResponse {return_amount: juno_sim, ..}) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.juno_wynd_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, juno_asset_info.clone())?;

                        let mut balance_msgs = balance_dao_msgs(
                        user_addr,
                        &project_addrs.destination_projects.balance_dao,
                        juno_sim,
                    )?;

                    balance_msgs.append_msgs(vec![juno_swap]);
                
                Ok(balance_msgs)},

                    JunoDestinationProject::MintLsd { lsd_type } => {
                        
                        let (juno_swap, wyndex::pair::SimulationResponse {return_amount: juno_sim, ..}) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.juno_wynd_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, juno_asset_info.clone())?;


                        let mut msgs = mint_juno_lsd_msgs(
                        user_addr,
                        lsd_type,
                        juno_sim,
                        project_addrs.destination_projects.juno_lsds.clone(),
                    )?; 
                    msgs.append_msgs(vec![juno_swap]);
                    
                    Ok(msgs)
                },
                    JunoDestinationProject::SparkIbcCampaign { fund } => {
                        let spark_addr = project_addrs.destination_projects.spark_ibc.fund.clone();

                        // swap on the usdc pair directly
                        let (usdc_swap,  wyndex::pair::SimulationResponse {return_amount: est_usdc,..}) = simulate_and_swap_wynd_pair(
                            &deps.querier, user_addr
                            , project_addrs.destination_projects.wynd.wynd_usdc_pair.as_ref(), 
                            Asset { info: wynd_asset_info.clone(), amount: comp_token_amount }, project_addrs.usdc.clone())?;
                            
                        // the helper method will validate that we have enough usdc and what not
                        let mut spark_msgs = spark_ibc_msgs(
                            user_addr,
                            &spark_addr,
                            cosmwasm_std::Coin {
                                denom: project_addrs.usdc.to_string(),
                                amount: est_usdc,
                            },
                            fund,
                        )?;

                        spark_msgs.prepend_msgs(vec![usdc_swap]);

                        Ok(spark_msgs)
                    }
                    JunoDestinationProject::SendTokens {
                        denom: target_asset,
                        address: to_address,
                    } => {
                        let (swap_msgs, sim) = wynd_wyndex_multihop_swap(
                            &deps.querier,
                            user_addr,
                            comp_token_amount,
                            wynd_asset_info.clone(),                            
                            target_asset.clone(),
                            project_addrs.destination_projects.wynd.multihop.to_string(),

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
