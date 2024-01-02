use crate::{
    dest_project_gen::DestinationResult,
    migaloo_comp_prefs::{AshAction, Denoms, MigalooProjectAddrs},
};
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use eris::{arb_vault::ExecuteMsg::Deposit, hub::ExecuteMsg::Bond};
use outpost_utils::{comp_prefs::CompoundPrefs, helpers::DestProjectMsgs, msg_gen::CosmosProtoMsg};
use std::fmt::Display;
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::migaloo_comp_prefs::MigalooDestinationProject;

/// Burns some number of WHALE tokens
pub fn burn_whale_msgs(
    user_addr: &T,
    whale_to_burn: Uint128,
    denoms: &Denoms,
    and_then: Option<AshAction>,
    project_addrs: &MigalooProjectAddrs,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    let mut burn_msgs = DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            project_addrs.furnace.to_string(),
            &user_addr.to_string(),
            &furnace::msg::ExecuteMsg::Burn {},
            Some(coins(whale_to_burn, denoms.whale)),
        )?)],
        events: vec![Event::new("burn_whale")
            .add_attribute("amount", whale_to_burn.to_string())
            .add_attribute("user", user_addr.to_string())],
    };

    if let Some(AshAction::EcosystemStake) = and_then {
        let eco_stake_msgs = ecosystem_stake_msgs(
            user_addr,
            Asset {
                amount: whale_to_stake,
                info: AssetInfo::NativeToken {
                    denom: denoms.ash.to_string(),
                },
            },
            denoms,
            &project_addrs.ecosystem_stake,
        )?;
        burn_msgs.sub_msgs.extend(eco_stake_msgs.sub_msgs);
        burn_msgs.msgs.extend(eco_stake_msgs.msgs);
        burn_msgs.events.extend(eco_stake_msgs.events);
    } else if let Some(AshAction::AmpAsh) = and_then {
        let amp_ash_msgs = eris_amp_vault_msgs(
            Asset {
                amount: whale_to_stake,
                info: AssetInfo::NativeToken {
                    denom: denoms.ash.to_string(),
                },
            },
            &project_addrs.ecosystem_stake,
        )?;
        burn_msgs.sub_msgs.extend(amp_ash_msgs.sub_msgs);
        burn_msgs.msgs.extend(amp_ash_msgs.msgs);
        burn_msgs.events.extend(amp_ash_msgs.events);
    }

    Ok(burn_msgs)
}

pub fn ecosystem_stake_msgs(
    user_addr: &T,
    asset: Asset,
    denoms: &Denoms,
    stake_contract: &Addr,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    // can only stake ash or musdc
    // TODO: add support for staking other assets based off some sort of query to the staking contract
    let msg: Vec<CosmosProtoMsg> = match asset.info {
        // delegating ash tokens
        AssetInfo::NativeToken { denom } if denom.eq(&denoms.ash) => {
            Ok(vec![CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    stake_contract,
                    &user_addr.to_string(),
                    &cw20_stake::msg::ReceiveMsg::Stake {},
                    Some(coins(asset.amount, denom)),
                )?,
            )])
        }
        // delegating musdc tokens which is a cw20
        AssetInfo::Token { contract_addr } if contract_addr.eq(&denoms.musdc) => {
            Ok(vec![CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    contract_addr.to_string(),
                    &user_addr.to_string(),
                    &cw20::Cw20ExecuteMsg::Send {
                        amount: asset.amount,
                        contract: stake_contract.to_string(),
                        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
                    },
                    None,
                )?,
            )])
        }
        _ => Err(DestinationError::InvalidAsset {
            asset: asset.to_string(),
            project: "ecosystem stake".to_string(),
        }),
    }?;

    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs,
        events: vec![Event::new("ecosystem_stake").add_attribute("asset", asset.info.to_string())],
    })
}

/// Creates the messages for any of the eris amplifier vaults (e.g. ampWHALE, ampASH, ampUSDC)
pub fn eris_amp_vault_msgs(asset: Asset, vault_addr: &Addr) -> DestProjectMsgs {
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(match asset {
            Asset {amount, info: AssetInfo::NativeToken {denom}} create_exec_contract_msg(
            vault_addr.to_string(),
            &user_addr.to_string(),
            &Bond { receiver: None },
            Some(coins(amount, denom)),
        ),
        Asset {amount, info: AssetInfo::Token {contract_addr}} create_exec_contract_msg(
            contract_addr.to_string(),
            &user_addr.to_string(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: vault_addr.to_string(),
                amount: amount,
                msg: to_binary(&Bond {receiver: None})?,
            },
            None,
        ),
    }?)],
        events: vec![Event::new("eris_amp_vault")
            .add_attribute("amount", asset.to_string())
            .add_attribute("user", user_addr.to_string())],
    })
}

/// Creates the messages for the eris arb vault (e.g. arbWHALE)
pub fn eris_arb_vault_msgs(asset: Asset, vault_addr: &Addr) -> DestProjectMsgs {
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            vault_addr.to_string(),
            &user_addr.to_string(),
            &Deposit {
                asset,
                receiver: None,
            },
            Some(coins(asset.amount, asset.info.denom())),
        )?)],
        events: vec![Event::new("eris_arb_vault")
            .add_attribute("amount", asset.to_string())
            .add_attribute("user", user_addr.to_string())],
    })
}

/// Stakes a given alliance asset via the alliance module
/// reference: https://github.com/terra-money/alliance-protocol/blob/e39d9648a5560a981b59ec9eacd8bc453d1500cb/contracts/alliance-hub/src/contract.rs#L342
pub fn alliance_stake_msgs(
    user_addr: &T,
    asset: Asset,
    denoms_list: &Denoms,
    validator_addr: &T,
) -> DestProjectMsgs
where
    T: Into<String> + Display,
{
    // can only stake ampluna or bluna
    match asset.info {
        AssetInfo::NativeToken { denom }
            if denom.eq(&denoms_list.ampluna) || denom.eq(&denoms_list.bluna) =>
        {
            Ok(DestProjectMsgs {
                msgs: vec![CosmosProtoMsg::AllianceDelegate(MsgDelegate {
                    delegator_address: user_addr.to_string(),
                    validator_address: validator_addr.to_string(),
                    amount: coins(asset.amount, denom),
                })],
                sub_msgs: vec![],
                events: vec![Event::new("alliance_stake")
                    .add_attribute("asset", asset.info.to_string())
                    .add_attribute("amount", asset.amount.to_string())],
            })
        }
        _ => Err(DestinationError::InvalidAsset {
            asset: asset.to_string(),
            project: "alliance stake".to_string(),
        }),
    }
}
