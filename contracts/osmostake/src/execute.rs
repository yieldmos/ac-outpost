use cosmwasm_std::{coin, coins, Addr, Attribute, Decimal, DepsMut, Env, Event, MessageInfo, Response, Timestamp};
use membrane_helpers::{
    msg_gen::{deposit_into_cdp_msgs, repay_cdt_msgs, stake_mbrn_msgs},
    utils::{ltv_in_range, membrane_deposit_collateral_and_then},
};
use osmosis_destinations::{
    comp_prefs::{
        OsmosisCompPrefs, OsmosisDepositCollateral, OsmosisDestinationProject, OsmosisLsd, OsmosisPoolSettings,
        OsmosisRepayDebt, RepayThreshold,
    },
    dest_project_gen::{mint_milk_tia_msgs, stake_ion_msgs},
    pools::MultipleStoredPools,
};
use osmosis_helpers::{
    osmosis_lp::{gen_join_cl_pool_single_sided_msgs, gen_join_classic_pool_single_sided_msgs},
    osmosis_swap::{
        estimate_token_out_min_amount, generate_known_to_known_swap_and_sim_msg, generate_known_to_unknown_route,
        generate_known_to_unknown_swap_and_sim_msg, generate_swap, OsmosisRoutePools,
    },
};
use outpost_utils::{
    comp_prefs::{CompoundPrefs, DestinationAction, TakeRate},
    helpers::{
        calculate_compound_amounts, combine_responses, is_authorized_compounder, prefs_sum_to_one, sum_coins,
        DestProjectMsgs,
    },
};
use sail_destinations::dest_project_gen::mint_eris_lsd_msgs;
use std::iter;
use universal_destinations::dest_project_gen::{native_staking_msg, send_tokens_msgs};
use white_whale::pool_network::asset::{Asset, AssetInfo};
use withdraw_rewards_tax_grant::{client::WithdrawRewardsTaxClient, msg::SimulateExecuteResponse};

use crate::{
    msg::ContractAddrs,
    state::{
        SubmsgData, ADMIN, AUTHORIZED_ADDRS, KNOWN_DENOMS, KNOWN_OSMO_POOLS, KNOWN_USDC_POOLS, SUBMSG_DATA, SUBMSG_REPLY_ID,
    },
    ContractError,
};

pub fn compound(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    project_addresses: ContractAddrs,
    user_address: String,
    comp_prefs: OsmosisCompPrefs,
    fee_to_charge: Option<Decimal>,
    TakeRate { .. }: TakeRate,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let user_addr: Addr = deps.api.addr_validate(&user_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &user_addr, ADMIN, AUTHORIZED_ADDRS)?;

    // get the denom of the staking token. this should be "uosmo"
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
    ) = WithdrawRewardsTaxClient::new(&project_addresses.authzpp.withdraw_tax, &user_addr)
        .simulate_with_contract_execute(deps.querier, fee_to_charge)?;

    let total_rewards = sum_coins(&staking_denom, &delegator_rewards);

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(
        &project_addresses,
        &user_addr,
        total_rewards.clone(),
        comp_prefs,
        &mut deps.branch(),
        env.block.time,
    )?;

    // prepare the response to the user with the withdraw rewards and other general metadata
    let withdraw_response = Response::default()
        .add_attribute("action", "outpost compound")
        .add_message(withdraw_msg)
        .add_attributes(vec![("subaction", "withdraw rewards"), ("user", &user_addr.to_string())])
        // event to track the amount of rewards that were automated
        .add_event(
            Event::new("amount_automated").add_attributes([total_rewards].iter().enumerate().map(|(i, coin)| Attribute {
                key: format!("amount_{}", i),
                value: coin.to_string(),
            })),
        );

    let resps = combine_responses(vec![
        withdraw_response,
        all_msgs
            .into_iter()
            .collect::<DestProjectMsgs>()
            .to_response(&env.contract.address)?,
    ]);

    Ok(resps)
}

/// Converts the user's compound preferences into a list of
/// CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    project_addrs: &ContractAddrs,
    user_addr: &Addr,
    total_rewards: cosmwasm_std::Coin,
    comp_prefs: OsmosisCompPrefs,
    deps_mut: &mut DepsMut<'_>,
    current_timestamp: Timestamp,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
    // let deps = deps_mut.branch().as_ref();
    let dca_denom = total_rewards.denom.clone();

    // calculates the amount of ujuno that will be used for each target project accurately.
    // these amounts are paired with the associated destination action
    // for example (1000, OsmosisDestinationProject::JunoStaking { validator_address: "juno1..." })
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
                        denom: dca_denom.clone(),
                    },
                    amount: comp_token_amount,
                };

                match destination {
                    OsmosisDestinationProject::OsmosisStaking { validator_address } => Ok(native_staking_msg(
                        &validator_address,
                        user_addr,
                        &cosmwasm_std::Coin {
                            denom: dca_denom.clone(),
                            amount: comp_token_amount,
                        },
                    )?),
                    OsmosisDestinationProject::TokenSwap { target_asset } => {
                        let route = generate_known_to_unknown_route(
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            "uosmo",
                            target_asset.clone(),
                        )?;

                        Ok(DestProjectMsgs {
                            msgs: vec![generate_swap(
                                &coin(comp_token_amount.u128(), "uosmo"),
                                user_addr,
                                route.clone(),
                                estimate_token_out_min_amount(
                                    &deps_mut.as_ref().querier,
                                    &route,
                                    "uosmo".to_string(),
                                    comp_token_amount,
                                    current_timestamp,
                                )?,
                            )],
                            sub_msgs: vec![],
                            events: vec![Event::new("token_swap").add_attribute("target_asset", target_asset.to_string())],
                        })
                    }
                    OsmosisDestinationProject::SendTokens {
                        address: to_address,
                        target_asset,
                    } => {
                        let (sim, swap_msgs) = generate_known_to_unknown_swap_and_sim_msg(
                            &deps_mut.querier,
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            user_addr,
                            &coin(comp_token_amount.u128(), "uosmo"),
                            target_asset.clone(),
                            current_timestamp,
                        )?;

                        // after the swap we can send the estimated funds to the target address
                        let mut send_msgs = send_tokens_msgs(
                            user_addr,
                            &deps_mut.api.addr_validate(&to_address)?,
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: target_asset.denom,
                                },
                                amount: sim,
                            },
                        )?;

                        send_msgs.prepend_msgs(swap_msgs);

                        Ok(send_msgs)
                    }
                    OsmosisDestinationProject::MintLsd { lsd: OsmosisLsd::Eris } => Ok(mint_eris_lsd_msgs(
                        user_addr,
                        compounding_asset,
                        &project_addrs.destination_projects.projects.eris_amposmo_bonding,
                    )?),

                    OsmosisDestinationProject::MintLsd {
                        lsd: OsmosisLsd::MilkyWay,
                    } => {
                        // swap OSMO to TIA
                        let (est_tia, swap_to_tia_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps_mut.querier,
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            user_addr,
                            &coin(comp_token_amount.u128(), "uosmo"),
                            &project_addrs.destination_projects.denoms.tia,
                            current_timestamp,
                        )?;

                        // Mint milkTIA
                        let mut mint_milk_tia = mint_milk_tia_msgs(
                            user_addr,
                            &project_addrs.destination_projects.projects.milky_way_bonding,
                            coin(est_tia.u128(), project_addrs.destination_projects.denoms.tia.clone()),
                        )?;

                        mint_milk_tia.prepend_msgs(swap_to_tia_msgs);

                        Ok(mint_milk_tia)
                    }
                    OsmosisDestinationProject::IonStaking {} => {
                        // swap OSMO to ION
                        let (est_ion, swap_to_ion_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps_mut.querier,
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            user_addr,
                            &coin(comp_token_amount.u128(), "uosmo"),
                            &project_addrs.destination_projects.denoms.ion,
                            current_timestamp,
                        )?;

                        let mut staking_msg =
                            stake_ion_msgs(user_addr, &project_addrs.destination_projects.projects.ion_dao, est_ion)?;

                        staking_msg.prepend_msgs(swap_to_ion_msgs);

                        Ok(staking_msg)
                    }
                    OsmosisDestinationProject::MembraneStake {} => {
                        // swap OSMO to MBRN
                        let (est_mbrn, swap_to_mbrn_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps_mut.querier,
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            user_addr,
                            &coin(comp_token_amount.u128(), "uosmo"),
                            &project_addrs.destination_projects.denoms.mbrn,
                            current_timestamp,
                        )?;

                        // can't stake less than 1 MBRN
                        if est_mbrn.u128().lt(&1_000_000u128) {
                            return Ok(DestProjectMsgs {
                                msgs: vec![],
                                sub_msgs: vec![],
                                events: vec![Event::new("membrane_stake")
                                    .add_attribute("skipped", "true")
                                    .add_attribute("amount", est_mbrn.to_string())],
                            });
                        }

                        let mut staking_msg = stake_mbrn_msgs(
                            user_addr,
                            &project_addrs.destination_projects.projects.membrane.staking,
                            coin(est_mbrn.u128(), project_addrs.destination_projects.denoms.mbrn.clone()),
                        )?;

                        staking_msg.prepend_msgs(swap_to_mbrn_msgs);

                        Ok(staking_msg)
                    }
                    // Entering tradition lp where we can use single asset lp
                    OsmosisDestinationProject::OsmosisLiquidityPool {
                        pool_id,
                        pool_settings: OsmosisPoolSettings::Standard { bond_tokens },
                    } => Ok(gen_join_classic_pool_single_sided_msgs(
                        &deps_mut.querier,
                        deps_mut.as_ref().storage,
                        OsmosisRoutePools {
                            stored_denoms: KNOWN_DENOMS,
                            stored_pools: MultipleStoredPools {
                                osmo: KNOWN_OSMO_POOLS,
                                usdc: KNOWN_USDC_POOLS,
                            },
                            pools: project_addrs.destination_projects.swap_routes.clone(),
                            denoms: project_addrs.destination_projects.denoms.clone(),
                        },
                        user_addr,
                        pool_id,
                        &coin(comp_token_amount.u128(), "uosmo"),
                        bond_tokens,
                        current_timestamp.clone(),
                    )?),
                    // Entering a CL pool
                    OsmosisDestinationProject::OsmosisLiquidityPool {
                        pool_id,
                        pool_settings:
                            OsmosisPoolSettings::ConcentratedLiquidity {
                                lower_tick,
                                upper_tick,
                                token_min_amount_0,
                                token_min_amount_1,
                            },
                    } => Ok(gen_join_cl_pool_single_sided_msgs(
                        &deps_mut.querier,
                        user_addr,
                        pool_id,
                        &coin(comp_token_amount.u128(), "uosmo"),
                        lower_tick,
                        upper_tick,
                        token_min_amount_0,
                        token_min_amount_1,
                        current_timestamp.clone(),
                    )?),

                    OsmosisDestinationProject::DepositCollateral {
                        as_asset,
                        protocol: OsmosisDepositCollateral::Membrane { position_id, and_then },
                    } => {
                        // TODO: this needs to be a sim and swap
                        let expected_deposits = coins(comp_token_amount.u128(), "uosmo");

                        Ok(match and_then.clone() {
                            // if there is no and_then action we just deposit the collateral and be done
                            None => deposit_into_cdp_msgs(
                                user_addr,
                                &project_addrs.destination_projects.projects.membrane.cdp,
                                position_id,
                                &expected_deposits,
                                None,
                            ),
                            // if there is a followup we likely will wind up spawning a submessage so there's more data to pass
                            Some(and_then) => membrane_deposit_collateral_and_then(
                                deps_mut.storage,
                                user_addr,
                                &project_addrs.destination_projects.projects.membrane.cdp,
                                position_id,
                                &expected_deposits,
                                &and_then,
                                SubmsgData::MintCdt {
                                    user_addr: user_addr.clone(),
                                    position_id,
                                    and_then: and_then.clone(),
                                },
                                SUBMSG_REPLY_ID,
                                SUBMSG_DATA,
                            ),
                        }?)
                    }
                    // repaying debt when the ltv has passed the threshold or there is no threshold set
                    // this means we should repay and be done with it
                    OsmosisDestinationProject::RepayDebt {
                        ltv_ratio_threshold: threshold,
                        protocol: OsmosisRepayDebt::Membrane { position_id },
                    } if ltv_in_range(
                        &deps_mut.querier,
                        &project_addrs.destination_projects.projects.membrane.cdp,
                        user_addr,
                        position_id,
                        threshold.clone(),
                    ) =>
                    {
                        // swap OSMO to CDT
                        let (est_cdt, swap_to_cdt_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps_mut.querier,
                            deps_mut.as_ref().storage,
                            OsmosisRoutePools {
                                stored_denoms: KNOWN_DENOMS,
                                stored_pools: MultipleStoredPools {
                                    osmo: KNOWN_OSMO_POOLS,
                                    usdc: KNOWN_USDC_POOLS,
                                },
                                pools: project_addrs.destination_projects.swap_routes.clone(),
                                denoms: project_addrs.destination_projects.denoms.clone(),
                            },
                            user_addr,
                            &coin(comp_token_amount.u128(), "uosmo"),
                            &project_addrs.destination_projects.denoms.cdt,
                            current_timestamp,
                        )?;

                        let mut repay_msgs = repay_cdt_msgs(
                            user_addr,
                            &project_addrs.destination_projects.projects.membrane.cdp,
                            position_id,
                            coin(est_cdt.u128(), project_addrs.destination_projects.denoms.cdt.clone()),
                        )?;

                        repay_msgs.prepend_msgs(swap_to_cdt_msgs);

                        Ok(repay_msgs)
                    }
                    // repaying debt when the ltv has not passed the threshold so we do the fallback
                    OsmosisDestinationProject::RepayDebt {
                        ltv_ratio_threshold: Some(RepayThreshold { otherwise, .. }),
                        ..
                    } => Ok(prefs_to_msgs(
                        project_addrs,
                        user_addr,
                        coin(comp_token_amount.u128(), "uosmo"),
                        CompoundPrefs {
                            relative: vec![DestinationAction {
                                destination: *otherwise,
                                amount: 1u128,
                            }],
                        },
                        deps_mut,
                        current_timestamp,
                    )?
                    .into_iter()
                    .collect::<DestProjectMsgs>()),
                    OsmosisDestinationProject::RepayDebt {
                        ltv_ratio_threshold: None,
                        ..
                    } => unimplemented!(
                        "this is already taken care of as the ltv in range qury is always true for 'No Threshold'"
                    ),
                    OsmosisDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
                }
            },
        )
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(compounding_msgs)
}
