use cosmwasm_std::{Addr, Decimal};
use outpost_utils::{
    helpers::RewardSplit,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use wynd_stake::msg::RewardsResponse;

use crate::helpers::gen_wynd_claim_rewards_msg;

#[test]
pub fn validate_wynd_rewards_split() {
    let delegator_addr = Addr::unchecked("delegator1");
    let tax_addr = Addr::unchecked("tax1");
    let wynd_addr = Addr::unchecked("wynd1");

    assert_eq!(
        gen_wynd_claim_rewards_msg(
            Decimal::percent(5),
            &delegator_addr,
            &tax_addr,
            &wynd_addr,
            RewardsResponse {
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
                        wynd_addr.to_string(),
                        &delegator_addr.clone(),
                        &wynd_stake::msg::ExecuteMsg::WithdrawRewards { owner: None, receiver: None },
                        None,
                    )
                    .unwrap()
                ),
                CosmosProtoMsg::ExecuteContract(
                    create_exec_contract_msg(
                        wynd_addr,
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
    )
}
