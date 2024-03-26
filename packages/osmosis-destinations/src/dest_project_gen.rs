use crate::{
    errors::OsmosisDestinationError,
    mars_types::{RedBankAction, RedBankExecuteMsgs},
    membrane_types::{MembraneStabilityPoolExecuteMsg, MembraneStakingExecuteMsg},
};

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Event, QuerierWrapper, ReplyOn, Uint128};
use membrane::{cdp, stability_pool, types::Basket};
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};

pub type DestinationResult = Result<DestProjectMsgs, OsmosisDestinationError>;

pub fn mint_milk_tia_msgs(
    minter_addr: &Addr,
    milk_tia_addr: &Addr,
    tia_to_bond: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            milk_tia_addr,
            minter_addr,
            &MilkyWayExecuteMsg::LiquidStake {},
            Some(vec![CsdkCoin {
                denom: tia_to_bond.denom.to_string(),
                amount: tia_to_bond.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![
            Event::new("mint_milk_tia").add_attribute("tia_to_bond", tia_to_bond.to_string())
        ],
    })
}

#[cw_serde]
pub enum MilkyWayExecuteMsg {
    LiquidStake {},
}

pub fn stake_ion_msgs(
    staker_addr: &Addr,
    ion_dao_addr: &Addr,
    ion_to_stake: Uint128,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            ion_dao_addr,
            staker_addr,
            &cw20_stake::msg::ReceiveMsg::Stake {},
            Some(vec![cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
                denom: "uion".to_string(),
                amount: ion_dao_addr.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("stake_ion").add_attribute("amount", ion_to_stake.to_string())],
    })
}

// fund account and lend the asset if wanted
pub fn fund_red_bank_acct_msgs(
    funder_addr: &Addr,
    funder_account_id: &str,
    redbank_addr: &Addr,
    fund_amount: Coin,
    lend_asset: bool,
) -> DestinationResult {
    // fund the account
    let mut actions: Vec<RedBankAction> = vec![RedBankAction::Deposit(fund_amount.clone())];

    // if the user wants to lend the asset add that action to the end
    if lend_asset {
        actions.push(RedBankAction::Lend((&fund_amount).into()));
    }

    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            redbank_addr,
            funder_addr,
            &RedBankExecuteMsgs::UpdateCreditAccount {
                account_id: funder_account_id.to_string(),
                actions,
            },
            Some(vec![CsdkCoin {
                denom: fund_amount.denom.to_string(),
                amount: fund_amount.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("fund_red_bank_acct")
            .add_attribute("fund_amount", fund_amount.to_string())
            .add_attribute("fund_and_lend", lend_asset.to_string())],
    })
}

/// stake mbrn
pub fn stake_mbrn_msgs(
    staker_addr: &Addr,
    staking_contract_addr: &Addr,
    mbrn_to_stake: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            staking_contract_addr,
            staker_addr,
            &membrane::staking::ExecuteMsg::Stake { user: None },
            Some(vec![CsdkCoin {
                denom: mbrn_to_stake.denom.to_string(),
                amount: mbrn_to_stake.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("stake_mbrn").add_attribute("amount", mbrn_to_stake.to_string())],
    })
}

/// deposit cdt into the stability pool
pub fn deposit_into_stability_pool_msgs(
    depositor_addr: &Addr,
    stability_pool_contract_addr: &Addr,
    cdt_to_deposit: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            stability_pool_contract_addr,
            depositor_addr,
            &stability_pool::ExecuteMsg::Deposit { user: None },
            Some(vec![CsdkCoin {
                denom: cdt_to_deposit.denom.to_string(),
                amount: cdt_to_deposit.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("deposit_into_stability_pool")
            .add_attribute("amount", cdt_to_deposit.to_string())],
    })
}

/// deposit basket assets into the user's CDP
pub fn deposit_into_cdp_msgs(
    depositor_addr: &Addr,
    cdp_contract_addr: &Addr,
    position_id: Uint128,
    deposits: &Vec<Coin>,
    as_submsg: Option<(u64, ReplyOn)>,
) -> DestinationResult {
    let deposit_msg = CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
        cdp_contract_addr,
        depositor_addr,
        &cdp::ExecuteMsg::Deposit {
            position_id: Some(position_id),
            position_owner: None,
        },
        Some(
            deposits
                .into_iter()
                .map(|coin| CsdkCoin {
                    denom: coin.denom.to_string(),
                    amount: coin.amount.to_string(),
                })
                .collect::<Vec<_>>(),
        ),
    )?);

    Ok(DestProjectMsgs {
        msgs: if as_submsg.is_none() {
            vec![]
        } else {
            vec![deposit_msg.clone()]
        },
        sub_msgs: if let Some((id, reply)) = as_submsg {
            vec![(id, vec![deposit_msg], reply)]
        } else {
            vec![]
        },
        events: vec![Event::new("deposit_into_cdp")
            .add_attribute(
                "deposits",
                deposits
                    .iter()
                    .map(|coin| coin.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .add_attribute("position_id", position_id.to_string())],
    })
}

/// filter out assets that are not in the CDP basket
pub fn basket_denoms_filter(
    querier: &QuerierWrapper,
    cdp_contract_addr: &Addr,
    assets: &Vec<Coin>,
) -> Result<Vec<Coin>, OsmosisDestinationError> {
    // check the currently allowed assets
    let basket: Basket =
        querier.query_wasm_smart(cdp_contract_addr, &cdp::QueryMsg::GetBasket {})?;

    Ok(assets
        .into_iter()
        // filter out assets that are not in the basket
        .filter(|coin| {
            basket
                .collateral_types
                .iter()
                // very weird checking Asset against Coin.denom. might work might blow up
                .any(|asset| asset.asset.info.to_string().eq(&coin.denom))
        })
        .cloned()
        .collect())
}

/// Mint CDT
pub fn mint_cdt_msgs(
    minter_addr: &Addr,
    cdp_contract_addr: &Addr,
    position_id: Uint128,
    desired_ltv: Decimal,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            cdp_contract_addr,
            minter_addr,
            &cdp::ExecuteMsg::IncreaseDebt {
                position_id,
                amount: None,
                LTV: Some(desired_ltv),
                mint_to_addr: None,
            },
            None,
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("mint_cdt")
            .add_attribute("desired_ltv", desired_ltv.to_string())
            .add_attribute("position_id", position_id.to_string())],
    })
}

/// Repay CDT
pub fn repay_cdt_msgs(
    repayer_addr: &Addr,
    cdp_contract_addr: &Addr,
    position_id: Uint128,
    repay_amount: Coin,
) -> DestinationResult {
    Ok(DestProjectMsgs {
        msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            cdp_contract_addr,
            repayer_addr,
            &cdp::ExecuteMsg::Repay {
                position_id,
                position_owner: None,
                send_excess_to: None,
            },
            Some(vec![CsdkCoin {
                denom: repay_amount.denom.to_string(),
                amount: repay_amount.amount.to_string(),
            }]),
        )?)],
        sub_msgs: vec![],
        events: vec![Event::new("repay_cdt")
            .add_attribute("repay_amount", repay_amount.to_string())
            .add_attribute("position_id", position_id.to_string())],
    })
}
