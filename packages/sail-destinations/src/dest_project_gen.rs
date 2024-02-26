use crate::{
    comp_prefs::{FundMsg, RacoonBetExec, RacoonBetGame, SparkIbcFund},
    errors::SailDestinationError,
};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Coin, Event, QuerierWrapper, Uint128,
};
use outpost_utils::{
    helpers::{csdk_coins, DestProjectMsgs},
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use std::fmt::Display;
use terraswap_helpers::terraswap_swap::{create_terraswap_pool_swap_msg, simulate_pool_swap};
use white_whale::pool_network::asset::{Asset, AssetInfo};

pub type DestinationResult = Result<DestProjectMsgs, SailDestinationError>;

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
                info: AssetInfo::NativeToken {
                    denom: bet.denom.clone(),
                },
            },
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

pub fn terraswap_pool_swap_msgs(
    user_addr: &Addr,
    pool_addr: &Addr,
    offer_asset: Asset,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![create_terraswap_pool_swap_msg(
            user_addr,
            offer_asset.clone(),
            pool_addr,
        )?],
        sub_msgs: vec![],
        events: vec![Event::new("terraswap_pool_swap")
            .add_attribute("offer", offer_asset.to_string())
            .add_attribute("pool", pool_addr.to_string())],
    })
}

pub fn terraswap_pool_swap_with_sim_msgs(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    pool_addr: &Addr,
    offer_asset: Asset,
) -> Result<(Uint128, DestProjectMsgs), SailDestinationError> {
    Ok((
        simulate_pool_swap(querier, &pool_addr.to_string(), &offer_asset)?.return_amount,
        DestProjectMsgs {
            msgs: vec![create_terraswap_pool_swap_msg(
                user_addr,
                offer_asset.clone(),
                pool_addr,
            )?],
            sub_msgs: vec![],
            events: vec![Event::new("terraswap_pool_swap")
                .add_attribute("offer", offer_asset.to_string())
                .add_attribute("pool", pool_addr.to_string())],
        },
    ))
}

pub fn mint_eris_lsd_msgs(user_addr: &Addr, amount: Asset, eris_addr: &Addr) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: match amount.clone() {
            Asset {
                info: AssetInfo::NativeToken { denom },
                amount,
            } => vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                eris_addr,
                user_addr,
                &bond_router::msg::ExecuteMsg::Bond {},
                Some(csdk_coins(&amount, denom)),
            )?)],
            Asset {
                info: AssetInfo::Token { contract_addr },
                amount,
            } => vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                contract_addr,
                user_addr,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: eris_addr.to_string(),
                    amount,
                    msg: to_json_binary(&bond_router::msg::ExecuteMsg::Bond {})?,
                },
                None,
            )?)],
        },
        sub_msgs: vec![],
        events: vec![Event::new("eris_lsd_mint").add_attribute("amount", amount.to_string())],
    })
}
