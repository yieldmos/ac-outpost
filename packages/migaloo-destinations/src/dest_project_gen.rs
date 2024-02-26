use crate::comp_prefs::{
    AshAction, Denoms, DestProjectVerifiedSwapRoutes, ErisMsg, GinkouEpochState, GinkouExecuteMsg,
    GinkouQueryMsg, MigalooProjectAddrs, WhaleLsd, WhaleLsdAddrs,
};
use crate::errors::MigalooDestinationError;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_std::{to_json_binary, Addr, Event, QuerierWrapper, Uint128};
use outpost_utils::helpers::csdk_coins;
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use sail_destinations::comp_prefs::{LsdQueryMsg, LsdStateResponse};
use sail_destinations::dest_project_gen::{mint_eris_lsd_msgs, terraswap_pool_swap_msgs};
use std::fmt::Display;
use terraswap_helpers::terraswap_swap::simulate_pool_swap;
use white_whale::pool_network::asset::{Asset, AssetInfo};

pub type DestinationResult = Result<DestProjectMsgs, MigalooDestinationError>;

/// Burns some number of WHALE tokens
pub fn burn_whale_msgs(
    user_addr: &Addr,
    whale_to_burn: Uint128,
    denoms: &Denoms,
    and_then: Option<AshAction>,
    project_addrs: &MigalooProjectAddrs,
) -> DestinationResult {
    let mut burn_msgs = DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            project_addrs.furnace.to_string(),
            &user_addr.to_string(),
            &furnace::msg::ExecuteMsg::Burn {},
            Some(csdk_coins(&whale_to_burn, denoms.whale.clone())),
        )?)],
        events: vec![Event::new("burn_whale")
            .add_attribute("amount", whale_to_burn.to_string())
            .add_attribute("user", user_addr.to_string())],
    };

    if let Some(AshAction::EcosystemStake) = and_then {
        let eco_stake_msgs = ecosystem_stake_msgs(
            user_addr,
            Asset {
                amount: whale_to_burn,
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
            user_addr,
            Asset {
                amount: whale_to_burn,
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

pub fn ecosystem_stake_msgs<T>(
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
    let msgs: Vec<CosmosProtoMsg> = match asset.info.clone() {
        // delegating ash tokens
        AssetInfo::NativeToken { denom } if denom.eq(&denoms.ash) => {
            Ok(vec![CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    stake_contract,
                    &user_addr.to_string(),
                    &cw20_stake::msg::ReceiveMsg::Stake {},
                    Some(csdk_coins(&asset.amount, denom)),
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
                        msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
                    },
                    None,
                )?,
            )])
        }
        _ => Err(MigalooDestinationError::InvalidAsset {
            denom: asset.to_string(),
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
pub fn eris_amp_vault_msgs(
    depositer_addr: &Addr,
    asset: Asset,
    vault_addr: &Addr,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        sub_msgs: vec![],
        msgs: vec![CosmosProtoMsg::ExecuteContract(match asset.clone() {
            Asset {
                amount,
                info: AssetInfo::NativeToken { denom },
            } => create_exec_contract_msg(
                vault_addr.to_string(),
                &depositer_addr.to_string(),
                &ErisMsg::Bond { receiver: None },
                Some(csdk_coins(&amount, denom)),
            ),
            Asset {
                amount,
                info: AssetInfo::Token { contract_addr },
            } => create_exec_contract_msg(
                contract_addr.to_string(),
                &depositer_addr.to_string(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: vault_addr.to_string(),
                    amount,
                    msg: to_json_binary(&ErisMsg::Bond { receiver: None })?,
                },
                None,
            ),
        }?)],
        events: vec![Event::new("eris_amp_vault")
            .add_attribute("amount", asset.to_string())
            .add_attribute("user", depositer_addr.to_string())],
    })
}

/// Creates the messages for the eris arb vault (e.g. arbWHALE)
pub fn eris_arb_vault_msgs(
    depositor_addr: &Addr,
    asset: Asset,
    vault_addr: &Addr,
) -> DestinationResult {
    if let Asset {
        amount,
        info: AssetInfo::NativeToken { denom },
    } = asset.clone()
    {
        Ok(DestProjectMsgs {
            sub_msgs: vec![],
            msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                vault_addr.to_string(),
                &depositor_addr.to_string(),
                &ErisMsg::Deposit {
                    asset: Asset {
                        amount: asset.amount,
                        info: AssetInfo::NativeToken {
                            denom: denom.clone(),
                        },
                    },
                    receiver: None,
                },
                Some(csdk_coins(&amount, denom)),
            )?)],
            events: vec![Event::new("eris_arb_vault")
                .add_attribute("amount", asset.to_string())
                .add_attribute("user", depositor_addr.to_string())],
        })
    } else {
        Err(MigalooDestinationError::InvalidAsset {
            denom: asset.to_string(),
            project: "eris arb vault".to_string(),
        })
    }
}

/// Stakes a given alliance asset via the alliance module
/// reference: https://github.com/terra-money/alliance-protocol/blob/e39d9648a5560a981b59ec9eacd8bc453d1500cb/contracts/alliance-hub/src/contract.rs#L342
pub fn alliance_stake_msgs<T, U>(
    user_addr: &T,
    asset: Asset,
    denoms_list: &Denoms,
    validator_addr: &U,
) -> DestinationResult
where
    T: Into<String> + Display,
    U: Into<String> + Display,
{
    // can only stake ampluna or bluna
    match asset.info.clone() {
        AssetInfo::NativeToken { denom }
            if denom.eq(&denoms_list.ampluna) || denom.eq(&denoms_list.bluna) =>
        {
            Ok(DestProjectMsgs {
                msgs: vec![CosmosProtoMsg::AllianceDelegate(MsgDelegate {
                    delegator_address: user_addr.to_string(),
                    validator_address: validator_addr.to_string(),
                    amount: Some(CsdkCoin {
                        amount: asset.amount.to_string(),
                        denom,
                    }),
                })],
                sub_msgs: vec![],
                events: vec![Event::new("alliance_stake")
                    .add_attribute("asset", asset.info.to_string())
                    .add_attribute("amount", asset.amount.to_string())],
            })
        }
        _ => Err(MigalooDestinationError::InvalidAsset {
            denom: asset.to_string(),
            project: "alliance stake".to_string(),
        }),
    }
}

/// Deposits Noble USDC to ginkou
pub fn deposit_ginkou_usdc_msgs<T>(
    user_addr: &T,
    amount: Uint128,
    denoms: &Denoms,
    ginkou_deposit_addr: &Addr,
) -> DestinationResult
where
    T: Into<String> + Display,
{
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            ginkou_deposit_addr.to_string(),
            &user_addr.to_string(),
            &GinkouExecuteMsg::DepositStable {},
            Some(csdk_coins(&amount, denoms.usdc.clone())),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("deposit_ginkou_usdc")
            .add_attribute("amount", amount.to_string())
            .add_attribute("user", user_addr.to_string())],
    })
}

/// Queries how much of the lsd will be minted given the amount of whale tokens supplied.
/// Returns an Asset of the LSD that will be minted
pub fn est_whale_lsd_mint(
    querier: &QuerierWrapper,
    whale_to_bond: Uint128,
    lsd: &WhaleLsd,
    lsd_addrs: &WhaleLsdAddrs,
    denoms: &Denoms,
) -> Result<Asset, MigalooDestinationError> {
    // query the state which will give us the exchange rate we need to estimate the mint
    let state: LsdStateResponse = querier
        .query_wasm_smart(lsd.get_mint_address(lsd_addrs), &LsdQueryMsg::State {})
        .map_err(|e| MigalooDestinationError::LsdMintEstimateError {
            error: e.to_string(),
            project: lsd.get_project_name(),
        })?;

    Ok(Asset {
        amount: state.exchange_rate * whale_to_bond,
        info: lsd.get_asset_info(denoms),
    })
}

/// Bond message for whale LSDs
pub fn mint_whale_lsd_msgs(
    user_addr: &Addr,
    lsd: &WhaleLsd,
    whale_to_bond: Uint128,
    lsd_addrs: &WhaleLsdAddrs,
    denoms: &Denoms,
) -> DestinationResult {
    Ok(mint_eris_lsd_msgs(
        user_addr,
        Asset {
            info: lsd.get_asset_info(denoms),
            amount: whale_to_bond,
        },
        &lsd.get_mint_address(lsd_addrs),
    )?)
}

pub fn mint_or_buy_whale_lsd_msgs(
    querier: &QuerierWrapper,
    user_addr: &Addr,
    lsd: &WhaleLsd,
    whale_to_bond: Uint128,
    lsd_addrs: &WhaleLsdAddrs,
    swap_routes: &DestProjectVerifiedSwapRoutes,
    denoms: &Denoms,
) -> Result<(Asset, DestProjectMsgs), MigalooDestinationError> {
    let whale_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        amount: whale_to_bond,
    };

    // check terraswap to see how much lsd we'd get for swapping
    let swap_est = simulate_pool_swap(
        querier,
        &lsd.get_whale_pool_addr(swap_routes).to_string(),
        &whale_asset,
    )?;

    // check the lsd provider to see how much lsd we'd get for bonding
    let mint_est = est_whale_lsd_mint(querier, whale_to_bond, lsd, lsd_addrs, denoms)?;

    // if the swap estimate is greater than the mint estimate, then we should swap
    if swap_est.return_amount.ge(&mint_est.amount) {
        Ok((
            Asset {
                info: lsd.get_asset_info(denoms),
                amount: swap_est.return_amount,
            },
            terraswap_pool_swap_msgs(
                user_addr,
                &lsd.get_whale_pool_addr(swap_routes),
                whale_asset,
            )?,
        ))
    }
    // otherwise we should be minting directly for a better rate
    else {
        Ok((
            mint_est,
            mint_whale_lsd_msgs(user_addr, lsd, whale_to_bond, lsd_addrs, denoms)?,
        ))
    }
}

pub fn query_ginkou_musdc_mint(
    querier: &QuerierWrapper,
    usdc_to_bond: Uint128,
    ginkou_addr: &Addr,
    denoms: &Denoms,
) -> Result<Asset, MigalooDestinationError> {
    let state: GinkouEpochState = querier
        .query_wasm_smart(ginkou_addr, &GinkouQueryMsg::EpochState {})
        .map_err(|e| MigalooDestinationError::ProjectQueryError {
            error: e.to_string(),
            project: "Ginkou - musdc epoch state".to_string(),
        })?;

    Ok(Asset {
        amount: usdc_to_bond.div_floor(state.exchange_rate),
        info: AssetInfo::Token {
            contract_addr: denoms.musdc.to_string(),
        },
    })
}
