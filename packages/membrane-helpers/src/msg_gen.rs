use crate::errors::MembraneHelperError;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CsdkCoin;
use cosmwasm_std::{Addr, Coin, Decimal, Event, ReplyOn, Uint128};
use membrane::{cdp, stability_pool};
use outpost_utils::{
    helpers::DestProjectMsgs,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};

pub type DestinationResult = Result<DestProjectMsgs, MembraneHelperError>;

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
