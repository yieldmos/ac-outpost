use cosmwasm_std::{Addr, Decimal};
use outpost_utils::{
    helpers::RewardSplit,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use wynd_stake::msg::WithdrawableRewardsResponse;

use crate::helpers::gen_wynd_claim_rewards_msg;

#[test]
pub fn validate_wynd_rewards_split() {
    let delegator_addr = Addr::unchecked("delegator1");
    let tax_addr = Addr::unchecked("tax1");
    let wynd_staking_addr = Addr::unchecked("wynd1");
    let wynd_cw20_addr = Addr::unchecked("wyndcw20");

    assert_eq!(
        gen_wynd_claim_rewards_msg(
            Decimal::percent(5),
            &delegator_addr,
            &tax_addr,
            &wynd_staking_addr,
            &wynd_cw20_addr,
            WithdrawableRewardsResponse {
                rewards: 1_000_000u128.into()
            }
        )
        .unwrap(),
        RewardSplit {
            user_rewards: 950_000u128.into(),
            tax_amount: 50_000u128.into(),
            claim_msgs: vec![
                CosmosProtoMsg::ExecuteContract(
                    create_exec_contract_msg(
                        wynd_staking_addr.to_string(),
                        &delegator_addr.clone(),
                        &wynd_stake::msg::ExecuteMsg::WithdrawRewards { owner: None, receiver: None },
                        None,
                    )
                    .unwrap()
                ),
                CosmosProtoMsg::ExecuteContract(
                    create_exec_contract_msg(
                        wynd_cw20_addr.clone(),
                        &delegator_addr.clone(),
                        &cw20::Cw20ExecuteMsg::Transfer {
                            recipient: tax_addr.to_string(),
                            amount: 50_000u128.into(),
                        },
                        None,
                    )
                    .unwrap()
                )
            ]
        },
        "Wynd rewards split is not correct"
    );

    assert_eq!(
        gen_wynd_claim_rewards_msg(
            Decimal::percent(0),
            &delegator_addr,
            &tax_addr,
            &wynd_staking_addr,
            &wynd_cw20_addr,
            WithdrawableRewardsResponse {
                rewards: 1_000_000u128.into()
            }
        )
        .unwrap(),
        RewardSplit {
            user_rewards: 1_000_000u128.into(),
            tax_amount: 0u128.into(),
            claim_msgs: vec![CosmosProtoMsg::ExecuteContract(
                create_exec_contract_msg(
                    wynd_staking_addr.to_string(),
                    &delegator_addr.clone(),
                    &wynd_stake::msg::ExecuteMsg::WithdrawRewards { owner: None, receiver: None },
                    None,
                )
                .unwrap()
            ),]
        },
        "Wynd rewards split shortcuts when 0 tax rate"
    )
}
