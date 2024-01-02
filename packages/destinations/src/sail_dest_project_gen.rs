use cosmos_sdk_proto::cosmos::{
    bank::v1beta1::MsgSend, base::v1beta1::Coin as CsdkCoin, staking::v1beta1::MsgDelegate,
};
use std::fmt::Display;

use cosmwasm_std::{Addr, Attribute, Coin, Event, QuerierWrapper, Uint128};
use outpost_utils::{
    helpers::DestProjectMsgs,

    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use white_whale::pool_network::asset::{Asset, AssetInfo};
use terraswap_helpers::terraswap_swap::simulate_pool_swap;
use crate::dest_project_gen::DestinationResult;
use crate::sail_comp_prefs::{FundMsg, RacoonBetExec, RacoonBetGame, SparkIbcFund};

/// pair address to use to check the bet size is gte 1 USDC
pub fn racoon_bet_msgs<T>(
    querier: &QuerierWrapper,
    player_address: &T,
    terraswap_usdc_pair_addr: Option<&Addr>,
    bet: Coin,
    game: RacoonBetGame,
    game_addr: &Addr,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    // can't use racoon bet unless the value of the play is at least $1 usdc
    if terraswap_usdc_pair_addr.is_some()
        && simulate_pool_swap(
            querier,
            // safe to unwrap this since we checked it above
            terraswap_usdc_pair_addr.unwrap().as_str(),
            &Asset {
                amount: bet.amount,
                info: AssetInfo::NativeToken { denom: bet.denom.clone()},
            },
            "usdc".to_string(),
        )?
        .return_amount
        .lt(&1_000_000u128.into())
        || (
            // otherwise we can assume we're receiving usdc and we can check the amount
            terraswap_usdc_pair_addr.is_none() && bet.amount.lt(&1_000_000u128.into())
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
