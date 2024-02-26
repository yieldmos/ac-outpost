use std::{iter, str::FromStr};

use cosmwasm_std::{coin, Addr, Attribute, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg, Uint128};
use osmosis_destinations::{
    comp_prefs::{OsmosisCompPrefs, OsmosisDestinationProject, OsmosisLsd, OsmosisPoolSettings},
    dest_project_gen::{mint_milk_tia_msgs, stake_ion_msgs, stake_mbrn_msgs},
    pools::MultipleStoredPools,
};
use osmosis_helpers::{
    osmosis_lp::{
        classic_pool_join_single_side_prepratory_swap, gen_join_cl_pool_single_sided_msgs,
        gen_join_classic_pool_single_sided_msgs, join_osmosis_pool_single_side, SingleSidedJoinSwap,
    },
    osmosis_swap::{
        generate_known_to_known_swap_and_sim_msg, generate_known_to_unknown_route,
        generate_known_to_unknown_swap_and_sim_msg, generate_swap, pool_swap_with_sim, OsmosisRoutePools,
    },
};
use osmosis_std::types::cosmos::base::v1beta1::Coin as OsmosisCoin;
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{
        calc_additional_tax_split, calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, sum_coins,
        DestProjectMsgs, TaxSplitResult,
    },
    msg_gen::create_exec_msg,
};
use sail_destinations::dest_project_gen::{mint_eris_lsd_msgs, spark_ibc_msgs, white_whale_satellite_msgs};
use terraswap_helpers::terraswap_swap::create_terraswap_swap_msg_with_simulation;
use universal_destinations::dest_project_gen::{daodao_cw20_staking_msg, native_staking_msg, send_tokens_msgs};
use white_whale::pool_network::asset::{Asset, AssetInfo};
use withdraw_rewards_tax_grant::{client::WithdrawRewardsTaxClient, msg::SimulateExecuteResponse};

use crate::{
    msg::ContractAddrs,
    state::{ADMIN, AUTHORIZED_ADDRS, KNOWN_DENOMS, KNOWN_OSMO_POOLS, KNOWN_USDC_POOLS, PROJECT_ADDRS},
    ContractError,
};

pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    project_addresses: ContractAddrs,
    user_address: String,
    comp_prefs: OsmosisCompPrefs,
    tax_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    // validate that the preference quantites sum to 1
    let _ = !prefs_sum_to_one(&comp_prefs)?;

    // check that the delegator address is valid
    let user_addr: Addr = deps.api.addr_validate(&user_address)?;

    // validate that the user is authorized to compound
    is_authorized_compounder(deps.as_ref(), &info.sender, &user_addr, ADMIN, AUTHORIZED_ADDRS)?;

    let project_addrs = PROJECT_ADDRS.load(deps.storage)?;

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
        .simulate_with_contract_execute(deps.querier, tax_fee)?;

    let total_rewards = sum_coins(&staking_denom, &delegator_rewards);

    // the list of all the compounding msgs to broadcast on behalf of the user based on their comp prefs
    let all_msgs = prefs_to_msgs(
        &project_addresses,
        // &env.block,
        // staking_denom,
        &user_addr,
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
    user_addr: &Addr,
    total_rewards: cosmwasm_std::Coin,
    comp_prefs: OsmosisCompPrefs,
    deps: Deps,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
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
                    OsmosisDestinationProject::TokenSwap { target_asset } => Ok(DestProjectMsgs {
                        msgs: vec![generate_swap(
                            &coin(comp_token_amount.u128(), "uosmo"),
                            user_addr,
                            generate_known_to_unknown_route(
                                deps.storage,
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
                            )?,
                        )],
                        sub_msgs: vec![],
                        events: vec![Event::new("token_swap").add_attribute("target_asset", target_asset.to_string())],
                    }),
                    OsmosisDestinationProject::SendTokens {
                        address: to_address,
                        target_asset,
                    } => {
                        let (sim, swap_msgs) = generate_known_to_unknown_swap_and_sim_msg(
                            &deps.querier,
                            deps.storage,
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
                        )?;

                        // after the swap we can send the estimated funds to the target address
                        let mut send_msgs = send_tokens_msgs(
                            user_addr,
                            &deps.api.addr_validate(&to_address)?,
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: target_asset.denom,
                                },
                                amount: sim,
                            },
                        )?;

                        send_msgs.append_msgs(swap_msgs);

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
                            &deps.querier,
                            deps.storage,
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
                        )?;

                        // Mint milkTIA
                        let mut mint_milk_tia = mint_milk_tia_msgs(
                            user_addr,
                            &project_addrs.destination_projects.projects.milky_way_bonding,
                            coin(est_tia.u128(), project_addrs.destination_projects.denoms.tia.clone()),
                        )?;

                        mint_milk_tia.append_msgs(swap_to_tia_msgs);

                        Ok(mint_milk_tia)
                    }
                    OsmosisDestinationProject::IonStaking {} => {
                        // swap OSMO to ION
                        let (est_ion, swap_to_ion_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps.querier,
                            deps.storage,
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
                        )?;

                        let mut staking_msg =
                            stake_ion_msgs(user_addr, &project_addrs.destination_projects.projects.ion_dao, est_ion)?;

                        staking_msg.prepend_msgs(swap_to_ion_msgs);

                        Ok(staking_msg)
                    }
                    OsmosisDestinationProject::MembraneStake {} => {
                        // swap OSMO to MBRN
                        let (est_mbrn, swap_to_mbrn_msgs) = generate_known_to_known_swap_and_sim_msg(
                            &deps.querier,
                            deps.storage,
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
                        &deps.querier,
                        deps.storage,
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
                        &deps.querier,
                        user_addr,
                        pool_id,
                        &coin(comp_token_amount.u128(), "uosmo"),
                        lower_tick,
                        upper_tick,
                        token_min_amount_0,
                        token_min_amount_1,
                    )?),

                    // OsmosisDestinationProject::RedBankLendAsset {
                    //     target_asset,
                    //     account_id,
                    // } => Ok(DestProjectMsgs::default()),
                    // OsmosisDestinationProject::WhiteWhaleSatellite { asset } => Ok(white_whale_satellite_msgs(
                    //     user_addr,
                    //     &project_addrs.destination_projects.projects.white_whale_satellite,
                    //     comp_token_amount,
                    // )?),
                    // OsmosisDestinationProject::MembraneRepay {
                    //     asset,
                    //     ltv_ratio_threshold,
                    // } => Ok(DestProjectMsgs::default()),
                    // OsmosisDestinationProject::MarginedRepay {
                    //     asset,
                    //     ltv_ratio_threshold,
                    // } => Ok(DestProjectMsgs::default()),
                    // OsmosisDestinationProject::MembraneDeposit { position_id, asset } => Ok(DestProjectMsgs::default()),

                    // OsmosisDestinationProject::DaoDaoStake { dao } => Ok(DestProjectMsgs::default()),
                    OsmosisDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
                }
            },
        )
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(compounding_msgs)
}
