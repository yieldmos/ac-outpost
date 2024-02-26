use outpost_utils::{
    helpers::RewardSplit,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg};

use wynd_helpers::wynd_swap::create_wyndex_swap_msg_with_simulation;
use wynd_stake::msg::WithdrawableRewardsResponse;
use wyndex::asset::AssetInfo;
use wyndex_multi_hop::msg::SwapOperation;

use crate::{msg::ExecuteMsg, ContractError};

// pub const WYND_CW20: &str = "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9";
// pub const WYND_ASSET_INFO: AssetInfo = AssetInfo::Token(WYND_CW20.to_string());
/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

pub fn query_and_generate_wynd_reward_msgs(
    tax_percent: Decimal,
    delegator_addr: &Addr,
    tax_addr: &Addr,
    wynd_staking_addr: &Addr,
    wynd_cw20_addr: &Addr,
    querier: &QuerierWrapper,
) -> Result<RewardSplit, ContractError> {
    gen_wynd_claim_rewards_msg(
        tax_percent,
        delegator_addr,
        tax_addr,
        wynd_staking_addr,
        wynd_cw20_addr,
        querier
            .query_wasm_smart(
                wynd_staking_addr,
                &wynd_stake::msg::QueryMsg::WithdrawableRewards {
                    owner: delegator_addr.to_string(),
                },
            )
            .map_err(|e| ContractError::QueryWyndRewardsFailure(e.to_string()))?,
    )
}

pub fn gen_wynd_claim_rewards_msg(
    tax_percent: Decimal,
    delegator_addr: &Addr,
    tax_addr: &Addr,
    wynd_staking_addr: &Addr,
    wynd_cw20_addr: &Addr,
    WithdrawableRewardsResponse { rewards }: WithdrawableRewardsResponse,
) -> Result<RewardSplit, ContractError> {
    if tax_percent.is_zero() {
        return Ok(RewardSplit {
            user_rewards: rewards,
            tax_amount: Uint128::zero(),
            claim_msgs: vec![CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
                wynd_staking_addr.to_string(),
                &delegator_addr,
                &wynd_stake::msg::ExecuteMsg::WithdrawRewards { owner: None, receiver: None },
                None,
            )?)],
        });
    }
    let user_rewards = rewards * (Decimal::one() - tax_percent);
    let tax_amount = rewards - user_rewards;

    let claim_msgs: Vec<CosmosProtoMsg> = vec![
        CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            wynd_staking_addr.to_string(),
            &delegator_addr,
            &wynd_stake::msg::ExecuteMsg::WithdrawRewards { owner: None, receiver: None },
            None,
        )?),
        CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            wynd_cw20_addr,
            delegator_addr,
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: tax_addr.to_string(),
                amount: tax_amount,
            },
            None,
        )?),
    ];

    Ok(RewardSplit {
        user_rewards,
        tax_amount,
        claim_msgs,
    })
}

pub fn wynd_wyndex_multihop_swap(
    querier: &QuerierWrapper,
    sender: &Addr,
    offer_amount: Uint128,
    wynd_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
    multihop_address: String,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), StdError> {
    let juno_asset_info = AssetInfo::Native("ujuno".to_string());

    create_wyndex_swap_msg_with_simulation(
        querier,
        sender,
        offer_amount,
        wynd_asset_info.clone(),
        ask_asset_info.clone(),
        multihop_address,
        Some(wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
            operations: vec![
                SwapOperation::WyndexSwap {
                    offer_asset_info: wynd_asset_info,
                    ask_asset_info: juno_asset_info.clone(),
                },
                SwapOperation::WyndexSwap {
                    offer_asset_info: juno_asset_info.clone(),
                    ask_asset_info: ask_asset_info.clone(),
                },
            ],
            minimum_receive: None,
            receiver: None,
            max_spread: Some(Decimal::percent(2)),
            referral_address: None,
            referral_commission: None,
        }),
    )
}
