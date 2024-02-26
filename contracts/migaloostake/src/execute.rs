use cosmwasm_std::{coin, Addr, Attribute, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg};
use migaloo_destinations::{
    comp_prefs::{
        DaoDaoStakingInfo, LsdMintAction, MUsdcAction, MigalooCompPrefs,
        MigalooDestinationProject, MigalooVault,
    },
    dest_project_gen::{
        burn_whale_msgs, deposit_ginkou_usdc_msgs, ecosystem_stake_msgs, eris_amp_vault_msgs, eris_arb_vault_msgs,
        mint_or_buy_whale_lsd_msgs, query_ginkou_musdc_mint,
    },
};
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, sum_coins, DestProjectMsgs},
    msg_gen::{create_exec_msg},
};
use std::iter;
use terraswap_helpers::terraswap_swap::{
    create_terraswap_pool_swap_msg_with_simulation,
};
use white_whale::pool_network::asset::{Asset, AssetInfo};

use withdraw_rewards_tax_grant::{client::WithdrawRewardsTaxClient, msg::SimulateExecuteResponse};

use crate::{
    msg::ContractAddrs,
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};
use sail_destinations::{
    dest_project_gen::{racoon_bet_msgs, spark_ibc_msgs, white_whale_satellite_msgs},
};
use universal_destinations::dest_project_gen::{daodao_staking_msg, native_staking_msg};

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    project_addresses: ContractAddrs,
    delegator_address: String,
    comp_prefs: MigalooCompPrefs,
    tax_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let delegator: Addr = deps.api.addr_validate(&delegator_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &delegator, ADMIN, AUTHORIZED_ADDRS)?;

    // get the denom of the staking token. this should be "ujuno"
    let staking_denom = project_addresses.staking_denom.clone();

    // prepare the withdraw rewards message and simulation from the authzpp grant
    let (
        SimulateExecuteResponse {
            // the rewards that the delegator is due to recieve
            delegator_rewards,
            ..
        },
        // withdraw delegator rewards wasm message
        withdraw_msg,
    ) = WithdrawRewardsTaxClient::new(&project_addresses.authzpp.withdraw_tax, &delegator)
        .simulate_with_contract_execute(deps.querier, tax_fee)?;

    let total_rewards = sum_coins(&staking_denom, &delegator_rewards);

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(
        &project_addresses,
        // &env.block,
        // staking_denom,
        &delegator,
        total_rewards.clone(),
        comp_prefs,
        deps.as_ref(),
    )?;

    let combined_msgs = all_msgs.iter().fold(DestProjectMsgs::default(), |mut acc, msg| {
        acc.msgs.append(&mut msg.msgs.clone());
        acc.sub_msgs.append(&mut msg.sub_msgs.clone());
        acc.events.append(&mut msg.events.clone());
        acc
    });

    let amount_automated_event =
        Event::new("amount_automated").add_attributes([total_rewards].iter().enumerate().map(|(i, coin)| Attribute {
            key: format!("amount_{}", i),
            value: coin.to_string(),
        }));

    // the final exec message that will be broadcast and contains all the sub msgs
    let exec_msg = create_exec_msg(&env.contract.address, combined_msgs.msgs)?;

    let resp = Response::default()
        .add_attribute("action", "outpost compound")
        .add_attribute("compoundee", delegator_address)
        .add_message(withdraw_msg)
        .add_attribute("subaction", "withdraw rewards")
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
    comp_prefs: MigalooCompPrefs,
    deps: Deps,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, MigalooDestinationProject::JunoStaking { validator_address: "juno1..." })
    let compound_token_amounts = iter::zip(
        calculate_compound_amounts(&comp_prefs.clone().try_into()?, &total_rewards.amount)?,
        comp_prefs.relative,
    );

    // generate the list of individual msgs to compound the user's rewards
    let compounding_msgs: Vec<DestProjectMsgs> = compound_token_amounts
        .map(
            |(comp_token_amount, DestinationAction { destination, .. })| -> Result<DestProjectMsgs, ContractError> {
                let compounding_asset = Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: comp_token_amount,
                };

                match destination {
                    MigalooDestinationProject::MigalooStaking { validator_address } => Ok(native_staking_msg(
                        &validator_address,
                        delegator_addr,
                        &cosmwasm_std::Coin {
                            denom: "uwhale".to_string(),
                            amount: comp_token_amount,
                        },
                    )?),

                    MigalooDestinationProject::DaoDaoStake { dao } => {
                        let DaoDaoStakingInfo {
                            dao_name,
                            dao_addr,
                            swap_pair_addr,
                            asset_info,
                        } = dao.staking_info(&project_addrs.destination_projects);

                        let (swap_msg, swap_sim) = create_terraswap_pool_swap_msg_with_simulation(
                            &deps.querier,
                            delegator_addr,
                            compounding_asset,
                            &swap_pair_addr,
                        )?;

                        let mut stake_msgs = daodao_staking_msg(
                            dao_name.to_string(),
                            delegator_addr,
                            &dao_addr,
                            Asset {
                                info: asset_info,
                                amount: swap_sim,
                            },
                        )?;

                        stake_msgs.prepend_msgs(vec![swap_msg]);

                        Ok(stake_msgs)
                    }

                    MigalooDestinationProject::Furnace { and_then } => Ok(burn_whale_msgs(
                        delegator_addr,
                        comp_token_amount,
                        &project_addrs.destination_projects.denoms,
                        and_then,
                        &project_addrs.destination_projects.projects,
                    )?),

                    MigalooDestinationProject::AllianceStake {
                        asset: _,
                        validator_address: _,
                    } => {
                        // let (swap_msg, asset) = match asset {
                        //     AllianceAsset::AmpLuna =>
                        //         project_addrs.destination_projects.swap_routes.whale.amp_luna
                        // };
                        unimplemented!()
                    }

                    MigalooDestinationProject::SparkIbcCampaign { fund } => {
                        let (swap_msg, est_usdc) = create_terraswap_pool_swap_msg_with_simulation(
                            &deps.querier,
                            delegator_addr,
                            compounding_asset,
                            &project_addrs.destination_projects.swap_routes.whale_usdc_pool,
                        )?;

                        let mut donate_msgs = spark_ibc_msgs(
                            delegator_addr,
                            &project_addrs.destination_projects.projects.spark_ibc,
                            coin(est_usdc.u128(), project_addrs.usdc.to_string()),
                            fund,
                        )?;

                        donate_msgs.prepend_msgs(vec![swap_msg]);

                        Ok(donate_msgs)
                    }

                    MigalooDestinationProject::RacoonBet { game } => Ok(racoon_bet_msgs(
                        &deps.querier,
                        delegator_addr,
                        Some(&project_addrs.destination_projects.swap_routes.whale_usdc_pool),
                        cosmwasm_std::Coin {
                            denom: "uwhale".to_string(),
                            amount: comp_token_amount,
                        },
                        game,
                        &project_addrs.destination_projects.projects.racoon_bet,
                    )?),
                    MigalooDestinationProject::WhiteWhaleSatellite { asset: _ } => {
                        // let (swap_msg, token_denom, est_token) = match asset {
                        //     AssetInfo::NativeToken { denom }
                        //         if denom.eq(&project_addrs.destination_projects.denoms.ampwhale) =>
                        //     {
                        //         Ok(todo!())
                        //     }
                        //     AssetInfo::NativeToken { denom }
                        //         if denom.eq(&project_addrs.destination_projects.denoms.bwhale) =>
                        //     {
                        //         Ok(todo!())
                        //     }
                        //     _ => Err(ContractError::SailDestinationError(SailDestinationError::InvalidAsset {
                        //         denom: asset.to_string(),
                        //         project: "white whale satellite market".to_string(),
                        //     })),
                        // }?;

                        // let (swap_msgs, sim) = create_terraswap_swap_msg_with_simulation(
                        //     &deps.querier,
                        //     delegator_addr,
                        //     comp_token_amount,
                        //     swap_ops,
                        //     project_addrs
                        //         .destination_projects
                        //         .projects
                        //         .terraswap_multihop_router
                        //         .to_string(),
                        // )?;

                        // let mut bond_msgs = white_whale_satellite_msgs(
                        //     delegator_addr,
                        //     coin(sim.u128(), asset.to_string()),
                        //     &project_addrs
                        //         .destination_projects
                        //         .projects
                        //         .white_whale_satellite
                        //         .market
                        //         .clone(),
                        // )?;

                        // bond_msgs.prepend_msgs(swap_msgs);

                        // Ok(bond_msgs)

                        unimplemented!()
                    }

                    MigalooDestinationProject::GinkouDepositUSDC { and_then } => {
                        let (swap_msg, est_usdc) = create_terraswap_pool_swap_msg_with_simulation(
                            &deps.querier,
                            delegator_addr,
                            compounding_asset,
                            &project_addrs.destination_projects.swap_routes.whale_usdc_pool,
                        )?;

                        let mut deposit_msgs = deposit_ginkou_usdc_msgs(
                            delegator_addr,
                            est_usdc,
                            &project_addrs.destination_projects.denoms,
                            &project_addrs.destination_projects.projects.ginkou.deposit,
                        )?;

                        deposit_msgs.prepend_msgs(vec![swap_msg]);

                        match and_then {
                            Some(MUsdcAction::EcosystemStake) => {
                                let est_musdc = query_ginkou_musdc_mint(
                                    &deps.querier,
                                    est_usdc,
                                    &project_addrs.destination_projects.projects.ginkou.deposit.clone(),
                                    &project_addrs.destination_projects.denoms,
                                )?;

                                let stake_msgs = ecosystem_stake_msgs(
                                    delegator_addr,
                                    est_musdc,
                                    &project_addrs.destination_projects.denoms,
                                    &project_addrs.destination_projects.projects.ecosystem_stake,
                                )?;

                                deposit_msgs.append_submsgs(stake_msgs.sub_msgs);
                                deposit_msgs.append_msgs(stake_msgs.msgs);
                                deposit_msgs.append_events(stake_msgs.events);

                                ()
                            }
                            Some(MUsdcAction::AmpUsdc) => {
                                let est_musdc = query_ginkou_musdc_mint(
                                    &deps.querier,
                                    est_usdc,
                                    &project_addrs.destination_projects.projects.ginkou.deposit.clone(),
                                    &project_addrs.destination_projects.denoms,
                                )?;

                                let bond_msgs = eris_amp_vault_msgs(
                                    delegator_addr,
                                    est_musdc,
                                    &project_addrs.destination_projects.projects.vaults.amp_usdc,
                                )?;

                                deposit_msgs.append_submsgs(bond_msgs.sub_msgs);
                                deposit_msgs.append_msgs(bond_msgs.msgs);
                                deposit_msgs.append_events(bond_msgs.events);

                                ()
                            }
                            None => (),
                        }

                        Ok(deposit_msgs)
                    }

                    MigalooDestinationProject::Vault {
                        vault: MigalooVault::ArbWhale,
                    } => Ok(eris_arb_vault_msgs(
                        delegator_addr,
                        compounding_asset,
                        &project_addrs.destination_projects.projects.whale_lsd.amp_whale,
                    )?),

                    MigalooDestinationProject::Vault {
                        vault: vault @ (MigalooVault::AmpAsh | MigalooVault::AmpUsdc),
                    } => {
                        let (swap_msg, bonding_asset, vault_addr) = match vault {
                            MigalooVault::AmpUsdc => {
                                // swap whale for usdc
                                let (swap_msg, est_usdc) = create_terraswap_pool_swap_msg_with_simulation(
                                    &deps.querier,
                                    delegator_addr,
                                    compounding_asset,
                                    &project_addrs.destination_projects.swap_routes.whale_usdc_pool,
                                )?;

                                (
                                    swap_msg,
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: project_addrs.destination_projects.denoms.usdc.to_string(),
                                        },
                                        amount: est_usdc,
                                    },
                                    &project_addrs.destination_projects.projects.vaults.amp_usdc,
                                )
                            }
                            MigalooVault::AmpAsh => {
                                // swap whale for ash this gets us more ash
                                let (swap_msg, est_usdc) = create_terraswap_pool_swap_msg_with_simulation(
                                    &deps.querier,
                                    delegator_addr,
                                    compounding_asset,
                                    &project_addrs.destination_projects.swap_routes.whale_ash_pool,
                                )?;

                                (
                                    swap_msg,
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: project_addrs.destination_projects.denoms.ash.to_string(),
                                        },
                                        amount: est_usdc,
                                    },
                                    &project_addrs.destination_projects.projects.vaults.amp_ash,
                                )
                            }
                            MigalooVault::ArbWhale => panic!("this should be handled in a previous match arm"),
                        };

                        let mut bond_msgs = eris_amp_vault_msgs(delegator_addr, bonding_asset, vault_addr)?;

                        bond_msgs.prepend_msgs(vec![swap_msg]);

                        Ok(bond_msgs)
                    }

                    MigalooDestinationProject::MintLsd { lsd_type, and_then } => {
                        let (mint_est, mint_msg) = mint_or_buy_whale_lsd_msgs(
                            &deps.querier,
                            delegator_addr,
                            &lsd_type,
                            comp_token_amount,
                            &project_addrs.destination_projects.projects.whale_lsd,
                            &project_addrs.destination_projects.swap_routes,
                            &project_addrs.destination_projects.denoms,
                        )?;

                        match and_then {
                            // mint and then bond to the sat market
                            Some(LsdMintAction::SatelliteMarket) => {
                                let mut satellite_bond_msgs = white_whale_satellite_msgs(
                                    delegator_addr,
                                    coin(mint_est.amount.u128(), mint_est.info.to_string()),
                                    &project_addrs.destination_projects.projects.white_whale_satellite.market,
                                )?;

                                satellite_bond_msgs.prepend_msgs(satellite_bond_msgs.msgs.clone());
                                satellite_bond_msgs.prepend_events(satellite_bond_msgs.events.clone());
                                satellite_bond_msgs.prepend_submsgs(satellite_bond_msgs.sub_msgs.clone());
                                Ok(mint_msg)
                            }
                            None => Ok(mint_msg),
                        }
                    }

                    MigalooDestinationProject::TokenSwap { target_denom: _ } => {
                        unimplemented!()
                    }
                    MigalooDestinationProject::SendTokens {
                        denom: _target_asset,
                        address: _to_address,
                    } => {
                        unimplemented!();
                        // let (swap_msgs, sim) = create_wyndex_swap_msg_with_simulation(
                        //     &deps.querier,
                        //     delegator_addr,
                        //     comp_token_amount,
                        //     AssetInfo::Native("ujuno".to_string()),
                        //     target_asset.clone(),
                        //     project_addrs.destination_projects.wynd.multihop.to_string(),
                        //     None,
                        // )
                        // .map_err(ContractError::Std)?;

                        // // after the swap we can send the estimated funds to the target address
                        // let mut send_msgs = send_tokens_msgs(
                        //     delegator_addr,
                        //     &deps.api.addr_validate(&to_address)?,
                        //     Asset {
                        //         info: target_asset,
                        //         amount: sim,
                        //     },
                        // )?;

                        // send_msgs.append_msgs(swap_msgs);

                        // Ok(send_msgs)
                    }
                    MigalooDestinationProject::GinkouRepayLoan {} => unimplemented!(),
                    MigalooDestinationProject::GinkouProvideLiquidity { asset: _, and_then: _ } => unimplemented!(),

                    MigalooDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
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
