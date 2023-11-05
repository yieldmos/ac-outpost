use std::iter;

use cosmos_sdk_proto::cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin, staking::v1beta1::MsgDelegate};
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, BlockInfo, Decimal, DepsMut, Env, Event, MessageInfo, QuerierWrapper, ReplyOn,
    Response, SubMsg, Uint128,
};
use outpost_utils::{
    comp_prefs::DestinationAction,
    helpers::{calculate_compound_amounts, is_authorized_compounder, prefs_sum_to_one, sum_coins},
    juno_comp_prefs::{
        GelottoExecute, JunoCompPrefs, JunoDestinationProject, JunoLsd, RacoonBetExec, RacoonBetGame, SparkIbcFund,
        WyndLPBondingPeriod, WyndStakingBondingPeriod,
    },
    msg_gen::{create_exec_contract_msg, create_exec_msg, CosmosProtoMsg},
};
use terraswap_helpers::terraswap_swap::create_terraswap_swap_msg_with_simulation;

use withdraw_rewards_tax_grant::{client::WithdrawRewardsTaxClient, msg::SimulateExecuteResponse};

use wynd_helpers::{
    wynd_lp::{wynd_join_pool_msgs, WyndAssetLPMessages},
    wynd_swap::{
        create_wyndex_swap_msg_with_simulation, simulate_and_swap_wynd_pair, simulate_wynd_pool_swap, wynd_pair_swap_msg,
    },
};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::{PairInfo, SimulationResponse},
};

use crate::{
    msg::ContractAddrs,
    queries::query_juno_wynd_swap,
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};

#[derive(Default)]
pub struct DestProjectMsgs {
    pub msgs: Vec<CosmosProtoMsg>,
    pub sub_msgs: Vec<(u64, Vec<CosmosProtoMsg>, ReplyOn)>,
    pub attributes: Vec<Attribute>,
}

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
        &env.block,
        staking_denom,
        &delegator,
        total_rewards.clone(),
        comp_prefs,
        deps.querier,
    )?;

    let combined_msgs = all_msgs.iter().fold(DestProjectMsgs::default(), |mut acc, msg| {
        acc.msgs.append(&mut msg.msgs.clone());
        acc.sub_msgs.append(&mut msg.sub_msgs.clone());
        acc.attributes.append(&mut msg.attributes.clone());
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
        .add_attributes(combined_msgs.attributes);

    Ok(resp)
}

/// Converts the user's compound preferences into a list of
/// CosmosProtoMsgs that will be broadcast on their behalf
pub fn prefs_to_msgs(
    project_addresses: &ContractAddrs,
    block: &BlockInfo,
    staking_denom: String,
    target_address: &Addr,
    total_rewards: cosmwasm_std::Coin,
    comp_prefs: JunoCompPrefs,
    querier: QuerierWrapper,
) -> Result<Vec<DestProjectMsgs>, ContractError> {
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
                    info: AssetInfo::Native(staking_denom.clone()),
                    amount: comp_token_amount,
                };
                let compounding_coin = Coin {
                    denom: staking_denom.clone(),
                    amount: comp_token_amount.into(),
                };
                match destination {
                    JunoDestinationProject::JunoStaking { validator_address } => Ok(DestProjectMsgs {
                        sub_msgs: vec![],
                        msgs: vec![CosmosProtoMsg::Delegate(MsgDelegate {
                            validator_address: validator_address.clone(),
                            amount: Some(Coin {
                                denom: total_rewards.denom.clone(),
                                amount: comp_token_amount.into(),
                            }),
                            delegator_address: target_address.to_string(),
                        })],
                        attributes: vec![
                            Attribute {
                                key: "subaction".to_string(),
                                value: "delegate juno".to_string(),
                            },
                            Attribute {
                                key: "validator".to_string(),
                                value: validator_address,
                            },
                        ],
                    }),
                    JunoDestinationProject::DaoStaking(dao) => {
                        let dao_addresses = dao.get_daos_addresses(&project_addresses.destination_projects.daos);

                        let (swap_msgs, expected_dao_token_amount) = if let Some(pair_addr) = dao_addresses.juno_wyndex_pair
                        {
                            // if there's a direct juno & staking denom pair, then we can swap directly
                            let (swap_msg, swap_sim) = simulate_and_swap_wynd_pair(
                                &querier,
                                target_address,
                                pair_addr.as_ref(),
                                compounding_asset,
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                            )?;

                            (vec![swap_msg], swap_sim.return_amount)
                        } else {
                            // otherwise we need to use the wyndex router to swap
                            create_wyndex_swap_msg_with_simulation(
                                &querier,
                                target_address,
                                comp_token_amount,
                                AssetInfo::Native(staking_denom.clone()),
                                AssetInfo::Token(dao_addresses.cw20.to_string()),
                                project_addresses.destination_projects.wynd.multihop.to_string(),
                            )?
                        };

                        // stake the tokens in the dao
                        let dao_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                            dao_addresses.cw20.clone(),
                            &target_address,
                            &cw20::Cw20ExecuteMsg::Send {
                                contract: dao_addresses.staking.to_string(),
                                amount: expected_dao_token_amount,
                                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
                            },
                            None,
                        )?);

                        Ok(DestProjectMsgs {
                            msgs: [swap_msgs, vec![dao_stake_msg]].concat(),
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "dao stake".to_string(),
                                },
                                Attribute {
                                    key: "type".to_string(),
                                    value: dao.to_string(),
                                },
                                Attribute {
                                    key: "amount".to_string(),
                                    value: expected_dao_token_amount.into(),
                                },
                            ],
                        })
                    }

                    JunoDestinationProject::WyndStaking { bonding_period } => {
                        let cw20 = project_addresses.destination_projects.wynd.cw20.clone();
                        let juno_wynd_pair = project_addresses.destination_projects.wynd.juno_wynd_pair.clone();

                        Ok(DestProjectMsgs {
                            msgs: wynd_staking_msgs(
                                cw20.as_ref(),
                                juno_wynd_pair.as_ref(),
                                target_address.clone(),
                                comp_token_amount,
                                staking_denom.clone(),
                                bonding_period.clone(),
                                query_juno_wynd_swap(juno_wynd_pair.as_ref(), &querier, comp_token_amount)?,
                            )?,
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "wynd staking".to_string(),
                                },
                                Attribute {
                                    key: "bonding_period".to_string(),
                                    value: u64::from(bonding_period).to_string(),
                                },
                            ],
                        })
                    }
                    JunoDestinationProject::TokenSwap { target_denom } => Ok(DestProjectMsgs {
                        msgs: wynd_helpers::wynd_swap::create_wyndex_swap_msg(
                            target_address,
                            comp_token_amount,
                            AssetInfo::Native(staking_denom.clone()),
                            target_denom,
                            project_addresses.destination_projects.wynd.multihop.to_string(),
                        )
                        .map_err(ContractError::Std)?,
                        sub_msgs: vec![],
                        attributes: vec![],
                    }),
                    JunoDestinationProject::WyndLp {
                        contract_address,
                        bonding_period,
                    } => {
                        // fetch the pool info so that we know how to do the swaps for entering the lp
                        let pool_info: wyndex::pair::PairInfo =
                            querier.query_wasm_smart(contract_address.to_string(), &wyndex::pair::QueryMsg::Pair {})?;

                        Ok(DestProjectMsgs {
                            msgs: join_wynd_pool_msgs(
                                project_addresses.destination_projects.wynd.multihop.to_string(),
                                &block.height,
                                &querier,
                                target_address.clone(),
                                comp_token_amount,
                                staking_denom.clone(),
                                contract_address,
                                bonding_period.clone(),
                                pool_info.clone(),
                                // checking the balance of the liquidity token to see if the user is already in the pool
                                querier.query_wasm_smart(
                                    pool_info.liquidity_token,
                                    &cw20::Cw20QueryMsg::Balance {
                                        address: target_address.to_string(),
                                    },
                                )?,
                            )?,
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "wynd lp".to_string(),
                                },
                                Attribute {
                                    key: "bonding_period".to_string(),
                                    value: u64::from(bonding_period).to_string(),
                                },
                            ],
                        })
                    }
                    JunoDestinationProject::GelottoLottery { lottery, lucky_phrase } => {
                        // 25k ujuno per ticket
                        let tickets_to_buy = comp_token_amount / Uint128::from(25_000u128);
                        Ok(DestProjectMsgs {
                            // if we dont have enough to buy a ticket, then we dont send any msgs
                            msgs: if tickets_to_buy.gt(&Uint128::zero()) {
                                vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                    lottery.get_lottery_address(&project_addresses.destination_projects.gelotto.clone()),
                                    target_address,
                                    &GelottoExecute::SenderBuySeed {
                                        referrer: Some(Addr::unchecked(project_addresses.take_rate_addr.clone())),
                                        count: u128::from(tickets_to_buy).clamp(0u128, u16::MAX as u128) as u16,
                                        seed: lucky_phrase,
                                    },
                                    Some(vec![Coin {
                                        amount: (tickets_to_buy * Uint128::from(25_000u128)).into(),
                                        denom: staking_denom.clone(),
                                    }]),
                                )?)]
                            } else {
                                vec![]
                            },
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "gelotto lottery".to_string(),
                                },
                                Attribute {
                                    key: "lottery".to_string(),
                                    value: lottery.to_string(),
                                },
                                Attribute {
                                    key: "tickets".to_string(),
                                    value: tickets_to_buy.to_string(),
                                },
                            ],
                        })
                    }
                    JunoDestinationProject::RacoonBet { game } => {
                        // can't use racoon bet unless the value of the play is at least $1 usdc
                        if simulate_wynd_pool_swap(
                            &querier,
                            project_addresses.destination_projects.racoon_bet.juno_usdc_wynd_pair.as_ref(),
                            &compounding_asset,
                            "usdc".to_string(),
                        )?
                        .return_amount
                        .lt(&1_000_000u128.into())
                        {
                            return Ok(DestProjectMsgs {
                                msgs: vec![],
                                sub_msgs: vec![],
                                attributes: vec![
                                    Attribute {
                                        key: "subaction".to_string(),
                                        value: "racoon bet".to_string(),
                                    },
                                    Attribute {
                                        key: "type".to_string(),
                                        value: "skipped".to_string(),
                                    },
                                ],
                            });
                        }

                        let (game, attributes) = match game {
                            RacoonBetGame::Slot { spins, .. } => {
                                let spin_value = comp_token_amount.checked_div(spins.into()).unwrap_or_default();
                                let msgs = RacoonBetGame::Slot {
                                    spins,
                                    spin_value,
                                    empowered: Uint128::zero(),
                                    free_spins: Uint128::zero(),
                                };
                                let attrs = vec![
                                    Attribute {
                                        key: "subaction".to_string(),
                                        value: "racoon bet".to_string(),
                                    },
                                    Attribute {
                                        key: "game".to_string(),
                                        value: game.to_string(),
                                    },
                                ];
                                (msgs, attrs)
                            }
                            RacoonBetGame::HundredSidedDice { selected_value } => (
                                RacoonBetGame::HundredSidedDice { selected_value },
                                vec![
                                    Attribute {
                                        key: "subaction".to_string(),
                                        value: "racoon bet".to_string(),
                                    },
                                    Attribute {
                                        key: "game".to_string(),
                                        value: game.to_string(),
                                    },
                                ],
                            ),
                        };

                        Ok(DestProjectMsgs {
                            msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                project_addresses.destination_projects.racoon_bet.game.clone(),
                                target_address,
                                &RacoonBetExec::PlaceBet { game },
                                Some(vec![compounding_coin]),
                            )?)],
                            sub_msgs: vec![],
                            attributes,
                        })
                    }
                    JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                        let swap_op = match asset.clone() {
                            AssetInfo::Native(denom)
                                if denom.eq(&project_addresses.destination_projects.white_whale.amp_whale) =>
                            {
                                Some(project_addresses.destination_projects.white_whale.juno_amp_whale_path.clone())
                            }
                            AssetInfo::Native(denom)
                                if denom.eq(&project_addresses.destination_projects.white_whale.amp_whale) =>
                            {
                                Some(
                                    project_addresses
                                        .destination_projects
                                        .white_whale
                                        .juno_bone_whale_path
                                        .clone(),
                                )
                            }
                            // if the asset isn't ampWHALE or bWhale then we can't do anything
                            _ => None,
                        };

                        if let (Some(swap_op), AssetInfo::Native(denom)) = (swap_op, asset) {
                            let (swap_msgs, sim) = create_terraswap_swap_msg_with_simulation(
                                &querier,
                                target_address,
                                comp_token_amount,
                                swap_op,
                                project_addresses
                                    .destination_projects
                                    .white_whale
                                    .terraswap_multihop_router
                                    .to_string(),
                            )?;

                            return Ok(DestProjectMsgs {
                                msgs: [
                                    swap_msgs,
                                    vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                        project_addresses.destination_projects.white_whale.market.clone(),
                                        target_address,
                                        &white_whale::whale_lair::Bond {
                                            asset: white_whale::pool_network::asset::Asset {
                                                amount: sim,
                                                info: white_whale::pool_network::asset::AssetInfo::NativeToken {
                                                    denom: denom.to_string(),
                                                },
                                            },
                                            timestamp: block.time,
                                            weight: Uint128::from(1u128),
                                        },
                                        Some(vec![Coin {
                                            denom: denom.to_string(),
                                            amount: sim.into(),
                                        }]),
                                    )?)],
                                ]
                                .concat(),
                                sub_msgs: vec![],
                                attributes: vec![
                                    Attribute {
                                        key: "subaction".to_string(),
                                        value: "white whale satellite".to_string(),
                                    },
                                    Attribute {
                                        key: "bonding_asset".to_string(),
                                        value: denom,
                                    },
                                ],
                            });
                        }
                        Ok(DestProjectMsgs {
                            msgs: vec![],
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "white whale satellite".to_string(),
                                },
                                Attribute {
                                    key: "type".to_string(),
                                    value: "skipped".to_string(),
                                },
                            ],
                        })
                    }
                    JunoDestinationProject::BalanceDao {} => Ok(DestProjectMsgs {
                        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                            project_addresses.destination_projects.balance_dao.clone(),
                            target_address,
                            &balance_token_swap::msg::ExecuteMsg::Swap {},
                            Some(vec![Coin {
                                denom: staking_denom.clone(),
                                amount: comp_token_amount.into(),
                            }]),
                        )?)],
                        sub_msgs: vec![
                        //     (
                        //     // disregard the result of the balance dao swap in case it fails
                        //     0u64,
                        //     vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                        //         project_addresses.destination_projects.balance_dao.clone(),
                        //         target_address,
                        //         &balance_token_swap::msg::ExecuteMsg::Swap {},
                        //         Some(vec![Coin {
                        //             denom: staking_denom.clone(),
                        //             amount: comp_token_amount.into(),
                        //         }]),
                        //     )?)],
                        //     ReplyOn::Error,
                        // )
                        ],
                        attributes: vec![
                            Attribute {
                                key: "subaction".to_string(),
                                value: "balance dao".to_string(),
                            },
                            Attribute {
                                key: "type".to_string(),
                                value: "mint balance".to_string(),
                            },
                        ],
                    }),
                    JunoDestinationProject::MintLsd { lsd_type } => {
                        let funds = Some(vec![Coin {
                            denom: staking_denom.clone(),
                            amount: comp_token_amount.into(),
                        }]);

                        let mint_msg = match lsd_type {
                            JunoLsd::StakeEasyB => create_exec_contract_msg(
                                project_addresses.destination_projects.juno_lsds.b_juno.clone(),
                                target_address,
                                &bjuno_token::msg::ExecuteMsg::Mint {
                                    recipient: target_address.to_string(),
                                    amount: comp_token_amount,
                                },
                                funds,
                            )?,
                            JunoLsd::StakeEasySe => create_exec_contract_msg(
                                project_addresses.destination_projects.juno_lsds.se_juno.clone(),
                                target_address,
                                &sejuno_token::msg::ExecuteMsg::Mint {
                                    recipient: target_address.to_string(),
                                    amount: comp_token_amount,
                                },
                                funds,
                            )?,
                            JunoLsd::Backbone =>
                            // not the type from the back bone contract but close enough
                            {
                                create_exec_contract_msg(
                                    project_addresses.destination_projects.juno_lsds.bone_juno.clone(),
                                    target_address,
                                    &bond_router::msg::ExecuteMsg::Bond {},
                                    funds,
                                )?
                            }
                            JunoLsd::Wynd => create_exec_contract_msg(
                                project_addresses.destination_projects.juno_lsds.wy_juno.clone(),
                                target_address,
                                &bond_router::msg::ExecuteMsg::Bond {},
                                funds,
                            )?,
                            JunoLsd::Eris =>
                            // not the type from the eris contract but close enough
                            {
                                create_exec_contract_msg(
                                    project_addresses.destination_projects.juno_lsds.amp_juno.clone(),
                                    target_address,
                                    &bond_router::msg::ExecuteMsg::Bond {},
                                    funds,
                                )?
                            }
                        };

                        Ok(DestProjectMsgs {
                            msgs: vec![CosmosProtoMsg::ExecuteContract(mint_msg)],
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "mint lsd".to_string(),
                                },
                                Attribute {
                                    key: "type".to_string(),
                                    value: lsd_type.to_string(),
                                },
                            ],
                        })
                    }
                    JunoDestinationProject::SparkIbcCampaign { fund } => {
                        let spark_addr = project_addresses.destination_projects.spark_ibc.fund.clone();

                        if let AssetInfo::Native(usdc_denom) = project_addresses.usdc.clone() {
                            let (mut swaps, est_donation) = create_wyndex_swap_msg_with_simulation(
                                &querier,
                                target_address,
                                comp_token_amount,
                                compounding_asset.info,
                                project_addresses.usdc.clone(),
                                project_addresses.destination_projects.wynd.multihop.to_string(),
                            )?;

                            if est_donation.lt(&Uint128::from(1_000_000u128)) {
                                return Ok(DestProjectMsgs::default());
                            }

                            swaps.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                spark_addr,
                                target_address,
                                &SparkIbcFund::Fund(fund),
                                Some(vec![Coin {
                                    denom: usdc_denom,
                                    amount: est_donation.into(),
                                }]),
                            )?));

                            Ok(DestProjectMsgs {
                                msgs: swaps,
                                sub_msgs: vec![],
                                attributes: vec![
                                    Attribute {
                                        key: "subaction".to_string(),
                                        value: "spark ibc".to_string(),
                                    },
                                    Attribute {
                                        key: "amount".to_string(),
                                        value: est_donation.to_string(),
                                    },
                                ],
                            })
                        } else {
                            Err(ContractError::NotImplemented {})
                        }
                    }
                    JunoDestinationProject::SendTokens {
                        denom: target_asset,
                        address: to_address,
                    } => {
                        let (mut swap_msgs, sim) = create_wyndex_swap_msg_with_simulation(
                            &querier,
                            target_address,
                            comp_token_amount,
                            AssetInfo::Native(staking_denom.clone()),
                            target_asset.clone(),
                            project_addresses.destination_projects.wynd.multihop.to_string(),
                        )
                        .map_err(ContractError::Std)?;

                        // after the swap we can send the estimated funds to the target address
                        swap_msgs.push(match &target_asset {
                            AssetInfo::Native(denom) => CosmosProtoMsg::Send(MsgSend {
                                amount: vec![Coin {
                                    denom: denom.clone(),
                                    amount: sim.into(),
                                }],
                                from_address: target_address.to_string(),
                                to_address: to_address.clone(),
                            }),
                            AssetInfo::Token(cw20_addr) => CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                                cw20_addr.clone(),
                                target_address,
                                &cw20::Cw20ExecuteMsg::Transfer {
                                    recipient: to_address.clone(),
                                    amount: sim,
                                },
                                None,
                            )?),
                        });

                        Ok(DestProjectMsgs {
                            msgs: swap_msgs,
                            sub_msgs: vec![],
                            attributes: vec![
                                Attribute {
                                    key: "subaction".to_string(),
                                    value: "send tokens".to_string(),
                                },
                                Attribute {
                                    key: "to_address".to_string(),
                                    value: to_address,
                                },
                                Attribute {
                                    key: "amount".to_string(),
                                    value: sim.to_string(),
                                },
                                Attribute {
                                    key: "denom".to_string(),
                                    value: target_asset.to_string(),
                                },
                            ],
                        })
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

pub fn neta_staking_msgs(
    neta_cw20_addr: &str,
    juno_neta_pair_addr: &str,
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    SimulationResponse {
        return_amount: expected_neta,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap juno for neta
    let neta_swap_msg = wynd_pair_swap_msg(
        &target_address,
        Asset {
            info: AssetInfo::Native(staking_denom),
            amount: comp_token_amount,
        },
        AssetInfo::Token(neta_cw20_addr.to_string()),
        juno_neta_pair_addr,
    )?;

    // stake neta
    let neta_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        neta_cw20_addr.to_string(),
        &target_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: neta_cw20_addr.into(),
            amount: expected_neta,
            msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
        },
        None,
    )?);

    Ok(vec![neta_swap_msg, neta_stake_msg])
}

pub fn wynd_staking_msgs(
    wynd_cw20_addr: &str,
    juno_wynd_pair_addr: &str,
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    bonding_period: WyndStakingBondingPeriod,
    SimulationResponse {
        return_amount: expected_wynd,
        ..
    }: SimulationResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // swap juno for wynd
    let wynd_swap_msg = wynd_pair_swap_msg(
        &target_address,
        Asset {
            info: AssetInfo::Native(staking_denom),
            amount: comp_token_amount,
        },
        AssetInfo::Token(wynd_cw20_addr.to_string()),
        juno_wynd_pair_addr,
    )?;

    // delegate wynd to the staking contract
    let wynd_stake_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        wynd_cw20_addr,
        &target_address,
        &cw20_vesting::ExecuteMsg::Delegate {
            amount: expected_wynd,
            msg: to_json_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                unbonding_period: bonding_period.into(),
            })?,
        },
        None,
    )?);

    Ok(vec![wynd_swap_msg, wynd_stake_msg])
}

#[allow(clippy::too_many_arguments)]
fn join_wynd_pool_msgs(
    wynd_multi_hop_address: String,
    _current_height: &u64,
    querier: &QuerierWrapper,
    target_address: Addr,
    comp_token_amount: Uint128,
    staking_denom: String,
    pool_contract_address: String,
    bonding_period: WyndLPBondingPeriod,
    pool_info: wyndex::pair::PairInfo,
    existing_lp_tokens: cw20::BalanceResponse,
) -> Result<Vec<CosmosProtoMsg>, ContractError> {
    // let pool_info: wyndex::pair::PoolResponse = querier.query_wasm_smart(
    //     pool_contract_address.to_string(),
    //     &wyndex::pair::QueryMsg::Pool {},
    // )?;

    // check the number of assets in the pool, but realistically this is expected to be 2
    let asset_count: u128 = pool_info.asset_infos.len().try_into().unwrap();

    // the amount of juno that will be used to swap for each asset in the pool
    let juno_amount_per_asset: Uint128 = comp_token_amount.checked_div_floor((asset_count, 1u128))?;

    // the list of prepared swaps and assets that will be used to join the pool
    let pool_assets = wynd_lp_asset_swaps(
        wynd_multi_hop_address,
        querier,
        &staking_denom,
        &juno_amount_per_asset,
        &pool_info,
        &target_address,
    )?;

    // the final list of swap messages that need to be executed before joining the pool is possible
    let mut swap_msgs: Vec<CosmosProtoMsg> =
        wynd_join_pool_msgs(target_address.to_string(), pool_contract_address, pool_assets)?;

    // as a temporary measure we bond the existing unbonded lp tokens- this is should
    // be resolved when wyndex updates itself
    // to add a bonding simulate function
    if !existing_lp_tokens.balance.is_zero() {
        swap_msgs.push(CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            pool_info.liquidity_token.to_string(),
            &target_address,
            &cw20::Cw20ExecuteMsg::Send {
                contract: pool_info.staking_addr.to_string(),
                amount: existing_lp_tokens.balance,
                msg: to_json_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                    unbonding_period: bonding_period.into(),
                })?,
            },
            None,
        )?));
    }

    Ok(swap_msgs)
    // will need to update things to utilize the routes from the factory
    // wyndex::factory::ROUTE;
}

/// Generates the wyndex swap messages and IncreaseAllowance (for cw20) messages
/// that are needed before the actual pool can be entered.
/// These messages should ensure that we have the correct amount of assets in the pool contract
pub fn wynd_lp_asset_swaps(
    wynd_multi_hop_address: String,
    querier: &QuerierWrapper,
    staking_denom: &str,
    wynd_amount_per_asset: &Uint128,
    pool_info: &PairInfo,
    target_address: &Addr,
) -> Result<Vec<WyndAssetLPMessages>, ContractError> {
    pool_info
        .asset_infos
        .iter()
        // map over each asset in the pool to generate the swap msgs and the target asset info
        .map(|asset| -> Result<WyndAssetLPMessages, ContractError> {
            let (swap_msgs, target_token_amount) = create_wyndex_swap_msg_with_simulation(
                querier,
                target_address,
                *wynd_amount_per_asset,
                AssetInfo::Token(staking_denom.to_string()),
                asset.clone().into(),
                wynd_multi_hop_address.to_string(),
            )?;

            Ok(WyndAssetLPMessages {
                swap_msgs,
                target_asset_info: Asset {
                    info: asset.clone().into(),
                    amount: target_token_amount,
                },
            })
        })
        .collect()
}
