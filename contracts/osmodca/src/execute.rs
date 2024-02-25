use std::iter;

use cosmwasm_std::{coin, Addr, Attribute, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Response, SubMsg};
use osmosis_destinations::{
    comp_prefs::{OsmosisCompPrefs, OsmosisDestinationProject, OsmosisLsd, OsmosisPoolSettings},
    dest_project_gen::{fund_red_bank_acct_msgs, mint_milk_tia_msgs, stake_ion_msgs},
};
use osmosis_helpers::osmosis_swap::pool_swap_with_sim;
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{
        calc_additional_tax_split, calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, DestProjectMsgs,
        TaxSplitResult,
    },
    msg_gen::create_exec_msg,
};
use sail_destinations::dest_project_gen::{mint_eris_lsd_msgs, spark_ibc_msgs, white_whale_satellite_msgs};
use terraswap_helpers::terraswap_swap::create_terraswap_swap_msg_with_simulation;
use universal_destinations::dest_project_gen::{daodao_cw20_staking_msg, native_staking_msg, send_tokens_msgs};
use white_whale::pool_network::asset::{Asset, AssetInfo};

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
        claim_and_tax_msgs: tax_store_msg,
    } = calc_additional_tax_split(
        compound_token,
        tax_fee.unwrap_or(Decimal::percent(1)),
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
            msgs: tax_store_msg,
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

                    // OsmosisDestinationProject::DaoDaoStake { dao } => Ok(DestProjectMsgs::default()),

                    // OsmosisDestinationProject::TokenSwap { target_asset } => unimplemented!("TokenSwap not implemented"),
                    // // OsmosisDestinationProject::TokenSwap { target_denom } => Ok(DestProjectMsgs {
                    // //     msgs: wynd_helpers::wynd_swap::create_wyndex_swap_msg(
                    // //         user_addr,
                    // //         comp_token_amount,
                    // //         AssetInfo::Native(dca_denom.clone()),
                    // //         target_denom,
                    // //         project_addrs.destination_projects.wynd.multihop.to_string(),
                    // //     )
                    // //     .map_err(ContractError::Std)?,
                    // //     sub_msgs: vec![],
                    // //     events: vec![],
                    // // }),
                    // OsmosisDestinationProject::MintLsd { lsd: OsmosisLsd::Eris } => Ok(mint_eris_lsd_msgs(
                    //     user_addr,
                    //     compounding_asset,
                    //     &project_addrs.destination_projects.projects.eris_amposmo_bonding,
                    // )?),

                    // OsmosisDestinationProject::MintLsd {
                    //     lsd: OsmosisLsd::MilkyWay,
                    // } => {
                    //     // swap OSMO to TIA
                    //     let (swap_to_tia_msgs, est_tia) = pool_swap_with_sim(
                    //         &deps.querier,
                    //         user_addr,
                    //         &project_addrs.destination_projects.swap_routes.osmo_tia_pool,
                    //         coin(comp_token_amount.u128(), dca_denom.clone()),
                    //         &project_addrs.destination_projects.denoms.tia,
                    //     )?;

                    //     // Mint milkTIA
                    //     let mut mint_milk_tia = mint_milk_tia_msgs(
                    //         user_addr,
                    //         &project_addrs.destination_projects.projects.milky_way_bonding,
                    //         coin(est_tia.u128(), project_addrs.destination_projects.denoms.tia.clone()),
                    //     )?;

                    //     mint_milk_tia.append_msgs(swap_to_tia_msgs);

                    //     Ok(mint_milk_tia)
                    // }

                    // OsmosisDestinationProject::MintLsd { lsd: OsmosisLsd::Eris } => Ok(mint_eris_lsd_msgs(
                    //     user_addr,
                    //     compounding_asset,
                    //     &project_addrs.destination_projects.projects.eris_amposmo_bonding,
                    // )?),

                    // OsmosisDestinationProject::SendTokens {
                    //     target_asset,
                    //     address: to_address,
                    // } => {
                    //     // let (swap_msgs, sim) = create_wyndex_swap_msg_with_simulation(
                    //     //     &deps.querier,
                    //     //     user_addr,
                    //     //     comp_token_amount,
                    //     //     AssetInfo::Native(dca_denom.clone()),
                    //     //     target_asset.clone(),
                    //     //     project_addrs.destination_projects.wynd.multihop.to_string(),
                    //     //     None,
                    //     // )
                    //     // .map_err(ContractError::Std)?;
                    //     let sim = 0u128.into();

                    //     // after the swap we can send the estimated funds to the target address
                    //     let mut send_msgs = send_tokens_msgs(
                    //         user_addr,
                    //         &deps.api.addr_validate(&to_address)?,
                    //         Asset {
                    //             info: AssetInfo::NativeToken { denom: target_asset },
                    //             amount: sim,
                    //         },
                    //     )?;

                    //     // send_msgs.append_msgs(swap_msgs);

                    //     Ok(send_msgs)
                    // }
                    // OsmosisDestinationProject::RedBankFundAccount {
                    //     account_id,
                    //     target_denom,
                    //     lend_asset,
                    // } => {
                    //     // TODO: implement the swap
                    //     // do a swap from osmo to `target_denom` and receive the sim
                    //     let (swap_msgs, simulated_target_amount) = (vec![], 0u128.into());
                    //     // pool_swap_with_sim(
                    //     //     &deps.querier,
                    //     //     user_addr,
                    //     //     &project_addrs.destination_projects.swap_routes.osmo_usdc_pool,
                    //     //     coin(comp_token_amount.u128(), dca_denom.clone()),
                    //     //     &target_denom,
                    //     // )?;

                    //     let mut funding_msgs = fund_red_bank_acct_msgs(
                    //         user_addr,
                    //         &account_id,
                    //         &project_addrs.destination_projects.projects.redbank.credit_manager.clone(),
                    //         coin(simulated_target_amount, target_denom),
                    //         lend_asset,
                    //     )?;

                    //     funding_msgs.prepend_msgs(swap_msgs);

                    //     Ok(DestProjectMsgs::default())
                    // }
                    // OsmosisDestinationProject::IonStaking {} => {
                    //     let (swap_msgs, ion_amount) = pool_swap_with_sim(
                    //         &deps.querier,
                    //         user_addr,
                    //         &project_addrs.destination_projects.swap_routes.osmo_ion_pool,
                    //         coin(comp_token_amount.u128(), dca_denom.clone()),
                    //         &project_addrs.destination_projects.denoms.ion,
                    //     )?;

                    //     let mut staking_msg =
                    //         stake_ion_msgs(user_addr, &project_addrs.destination_projects.projects.ion_dao, ion_amount)?;

                    //     staking_msg.prepend_msgs(swap_msgs);

                    //     Ok(staking_msg)
                    // }
                    // OsmosisDestinationProject::RedBankDeposit { target_denom } => Ok(DestProjectMsgs::default()),
                    // OsmosisDestinationProject::OsmosisLiquidityPool { pool_id, pool_settings } => {
                    //     match pool_settings {
                    //         OsmosisPoolSettings::Standard { bond_tokens } => {}
                    //         OsmosisPoolSettings::ConcentratedLiquidity {
                    //             lower_tick,
                    //             upper_tick,
                    //             token_min_amount_0,
                    //             token_min_amount_1,
                    //         } => {
                    //             unimplemented!()
                    //         }
                    //     }
                    //     Ok(DestProjectMsgs::default())
                    // }
                    OsmosisDestinationProject::Unallocated {} => Ok(DestProjectMsgs::default()),
                    _ => unimplemented!(),
                }
            },
        )
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(compounding_msgs)
}
