use cosmos_sdk_proto::cosmos::{
    bank::v1beta1::MsgSend, base::v1beta1::Coin as CsdkCoin, staking::v1beta1::MsgDelegate,
};
use cosmwasm_std::{to_json_binary, Addr, Attribute, Coin, Event, QuerierWrapper, Uint128};
use outpost_utils::{
    helpers::DestProjectMsgs,
    juno_comp_prefs::{
        FundMsg, GelottoAddrs, GelottoExecute, GelottoLottery, JunoLsd, JunoLsdAddrs,
        RacoonBetExec, RacoonBetGame, SparkIbcFund, StakeEasyMsgs, WyndStakingBondingPeriod,
    },
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use std::fmt::Display;
use wynd_helpers::wynd_swap::simulate_wynd_pool_swap;
use wyndex::asset::{Asset, AssetInfo};

use crate::errors::JunoHelperError;

type DestinationResult = Result<DestProjectMsgs, JunoHelperError>;

/// Generates the messages needed to delegate to a validator
pub fn native_staking_msg(
    validator_address: &str,
    delegator_addr: &(impl Into<String> + Display),
    coin: &Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::Delegate(MsgDelegate {
            validator_address: validator_address.to_string(),
            amount: Some(CsdkCoin {
                denom: coin.denom.clone(),
                amount: coin.amount.into(),
            }),
            delegator_address: delegator_addr.to_string(),
        })],
        events: vec![Event::new("delegate")
            .add_attribute("validator", validator_address)
            .add_attribute("amount", coin.to_string())],
    })
}

pub fn daodao_cw20_staking_msg<T>(
    dao_name: String,
    staker_address: &T,
    cw20_addr: &T,
    staking_contract_addr: &T,
    staking_amount: Uint128,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            cw20_addr.to_string(),
            &staker_address.to_string(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: staking_contract_addr.to_string(),
                amount: staking_amount,
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
            },
            None,
        )?)],
        events: vec![Event::new("dao_stake")
            .add_attribute("dao", dao_name.to_string())
            .add_attribute("amount", staking_amount.to_string())],
    })
}

pub fn wynd_staking_msgs<T>(
    wynd_cw20_addr: &T,
    staker_address: &T,
    staking_amount: Uint128,
    bonding_period: WyndStakingBondingPeriod,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            wynd_cw20_addr.to_string(),
            &staker_address.to_string(),
            &cw20_vesting::ExecuteMsg::Delegate {
                amount: staking_amount,
                msg: to_json_binary(&wynd_stake::msg::ReceiveDelegationMsg::Delegate {
                    unbonding_period: bonding_period.clone().into(),
                })?,
            },
            None,
        )?)],
        events: vec![Event::new("wynd_stake")
            .add_attribute("bonding_period", u64::from(bonding_period).to_string())
            .add_attribute("amount", staking_amount.to_string())],
    })
}

pub fn gelotto_lottery_msgs<T>(
    player_address: &T,
    ymos_referrer_addr: Addr,
    lottery: GelottoLottery,
    gelotto_addrs: &GelottoAddrs,
    lucky_phrase: u32,
    juno_amount: Uint128,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    // 25k ujuno per ticket
    let tickets_to_buy = juno_amount / Uint128::from(25_000u128);

    Ok(DestProjectMsgs {
        // if we dont have enough to buy a ticket, then we dont send any msgs
        msgs: if tickets_to_buy.gt(&Uint128::zero()) {
            vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                lottery.get_lottery_address(gelotto_addrs),
                player_address,
                &GelottoExecute::SenderBuySeed {
                    referrer: Some(ymos_referrer_addr),
                    count: u128::from(tickets_to_buy).clamp(0u128, u16::MAX as u128) as u16,
                    seed: lucky_phrase,
                },
                Some(vec![CsdkCoin {
                    amount: (tickets_to_buy * Uint128::from(25_000u128)).into(),
                    denom: "ujuno".to_string(),
                }]),
            )?)]
        } else {
            vec![]
        },
        sub_msgs: vec![],
        events: vec![Event::new("gelotto_lottery")
            .add_attribute("lottery", lottery.to_string())
            .add_attribute("tickets", tickets_to_buy)],
    })
}

/// pair address to use to check the bet size is gte 1 USDC
pub fn racoon_bet_msgs<T>(
    querier: &QuerierWrapper,
    player_address: &T,
    wyndex_usdc_pair_addr: Option<&Addr>,
    bet: Coin,
    game: RacoonBetGame,
    game_addr: &Addr,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    // can't use racoon bet unless the value of the play is at least $1 usdc
    if wyndex_usdc_pair_addr.is_some()
        && simulate_wynd_pool_swap(
            querier,
            // safe to unwrap this since we checked it above
            wyndex_usdc_pair_addr.unwrap().as_str(),
            &Asset {
                amount: bet.amount,
                info: wyndex::asset::AssetInfo::Native(bet.denom.clone()),
            },
            "usdc".to_string(),
        )?
        .return_amount
        .lt(&1_000_000u128.into())
        || (
            // otherwise we can assume we're receiving usdc and we can check the amount
            wyndex_usdc_pair_addr.is_none() && bet.amount.lt(&1_000_000u128.into())
        )
    {
        return Ok(DestProjectMsgs {
            msgs: vec![],
            sub_msgs: vec![],
            events: vec![Event::new("racoon_bet")
                .add_attribute("game", game.to_string())
                .add_attribute("type", "skipped")],
        });
    }

    let (game, game_amount, attributes) = match game {
        RacoonBetGame::Slot { spins, .. } => {
            let spin_value = bet.amount.checked_div(spins.into()).unwrap_or_default();
            let msgs = RacoonBetGame::Slot {
                spins,
                spin_value,
                empowered: Uint128::zero(),
                free_spins: Uint128::zero(),
            };
            let attrs = vec![Attribute {
                key: "game".to_string(),
                value: msgs.to_string(),
            }];
            (
                msgs,
                Coin {
                    denom: bet.denom,
                    amount: (spin_value * Uint128::from(spins as u128)),
                },
                attrs,
            )
        }
        RacoonBetGame::HundredSidedDice { selected_value } => (
            RacoonBetGame::HundredSidedDice { selected_value },
            bet,
            vec![Attribute {
                key: "game".to_string(),
                value: game.to_string(),
            }],
        ),
    };

    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            game_addr,
            player_address,
            &RacoonBetExec::PlaceBet { game },
            Some(vec![CsdkCoin {
                amount: game_amount.amount.into(),
                denom: game_amount.denom,
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("racoon_bet").add_attributes(attributes)],
    })
}

/// Generates the messages needed to bond to the white whale sat market
/// Assumes that youre passing a valid denom that's either bWHALE or ampWHALE
pub fn white_whale_satellite_msgs<T>(
    user_addr: &T,
    bond_token: Coin,
    market_addr: &T,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            market_addr.to_string(),
            user_addr,
            &white_whale::whale_lair::ExecuteMsg::Bond {
                asset: white_whale::pool_network::asset::Asset {
                    amount: bond_token.amount,
                    info: white_whale::pool_network::asset::AssetInfo::NativeToken {
                        denom: bond_token.denom.to_string(),
                    },
                },
                // timestamp: block.time,
                // weight: Uint128::from(1u128),
            },
            Some(vec![CsdkCoin {
                denom: bond_token.denom.clone(),
                amount: bond_token.amount.into(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![
            Event::new("white_whale_satellite").add_attribute("asset", bond_token.to_string())
        ],
    })
}

pub fn balance_dao_msgs<T>(
    user_addr: &T,
    dao_contract_addr: &T,
    juno_amount: Uint128,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            dao_contract_addr.to_string(),
            user_addr,
            &balance_token_swap::msg::ExecuteMsg::Swap {},
            Some(vec![CsdkCoin {
                denom: "ujuno".to_string(),
                amount: juno_amount.into(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("balance_dao").add_attribute("amount", juno_amount.to_string())],
    })
}

pub fn mint_juno_lsd_msgs<T>(
    user_addr: &T,
    lsd: JunoLsd,
    juno_to_bond: Uint128,
    lsd_addrs: JunoLsdAddrs,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    let funds = Some(vec![CsdkCoin {
        denom: "ujuno".to_string(),
        amount: juno_to_bond.into(),
    }]);

    if let JunoLsd::StakeEasyB | JunoLsd::StakeEasySe = lsd {
        if juno_to_bond.lt(&1_000_000u128.into()) {
            // if the amount is less than 1 JUNO then we don't mint
            return Ok(DestProjectMsgs {
                msgs: vec![],
                sub_msgs: vec![],
                events: vec![Event::new("mint_lsd")
                    .add_attribute("type", format!("{} skipped", lsd))
                    .add_attribute("amount", juno_to_bond.to_string())],
            });
        }
    }

    let mint_msg = match lsd {
        JunoLsd::StakeEasyB => create_exec_contract_msg(
            lsd_addrs.b_juno.to_string(),
            user_addr,
            &StakeEasyMsgs::StakeForBjuno { referral: 0 },
            funds,
        )?,
        JunoLsd::StakeEasySe => create_exec_contract_msg(
            lsd_addrs.se_juno.to_string(),
            user_addr,
            &StakeEasyMsgs::Stake { referral: 0 },
            funds,
        )?,
        JunoLsd::Backbone =>
        // not the type from the back bone contract but close enough
        {
            create_exec_contract_msg(
                lsd_addrs.bone_juno.to_string(),
                user_addr,
                &bond_router::msg::ExecuteMsg::Bond {},
                funds,
            )?
        }
        JunoLsd::Wynd => create_exec_contract_msg(
            lsd_addrs.wy_juno.to_string(),
            user_addr,
            &bond_router::msg::ExecuteMsg::Bond {},
            funds,
        )?,
        JunoLsd::Eris =>
        // not the type from the eris contract but close enough
        {
            create_exec_contract_msg(
                lsd_addrs.amp_juno.to_string(),
                user_addr,
                &bond_router::msg::ExecuteMsg::Bond {},
                funds,
            )?
        }
    };

    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(mint_msg)],
        sub_msgs: vec![],
        events: vec![Event::new("mint_lsd")
            .add_attribute("type", lsd.to_string())
            .add_attribute("amount", juno_to_bond.to_string())],
    })
}

pub fn spark_ibc_msgs<T>(
    donor_addr: &T,
    spark_addr: &Addr,
    usdc_donation: Coin,
    fund: FundMsg,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    if usdc_donation.amount.lt(&Uint128::from(1_000_000u128)) {
        return Ok(DestProjectMsgs {
            msgs: vec![],
            sub_msgs: vec![],
            events: vec![Event::new("spark_ibc")
                .add_attribute("type", "skipped")
                .add_attribute("amount", usdc_donation.to_string())],
        });
    }

    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            spark_addr,
            donor_addr,
            &SparkIbcFund::Fund(fund),
            Some(vec![CsdkCoin {
                denom: usdc_donation.denom.clone(),
                amount: usdc_donation.amount.into(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("spark_ibc").add_attribute("amount", usdc_donation.to_string())],
    })
}

pub fn send_tokens_msgs<T>(
    sender_addr: &T,
    recipient_addr: &T,
    asset_to_send: Asset,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        msgs: vec![match &asset_to_send.info {
            AssetInfo::Native(denom) => CosmosProtoMsg::Send(MsgSend {
                amount: vec![CsdkCoin {
                    denom: denom.clone(),
                    amount: asset_to_send.amount.into(),
                }],
                from_address: sender_addr.to_string(),
                to_address: recipient_addr.to_string(),
            }),
            AssetInfo::Token(cw20_addr) => {
                CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                    cw20_addr.clone(),
                    sender_addr,
                    &cw20::Cw20ExecuteMsg::Transfer {
                        recipient: recipient_addr.to_string(),
                        amount: asset_to_send.amount,
                    },
                    None,
                )?)
            }
        }],
        sub_msgs: vec![],
        events: vec![Event::new("send_tokens")
            .add_attribute("to_address", recipient_addr.to_string())
            .add_attribute("amount", asset_to_send.amount.to_string())
            .add_attribute("asset", asset_to_send.info.to_string())],
    })
}
