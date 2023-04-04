use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgWithdrawDelegatorReward;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::{authz::v1beta1::MsgExec, base::v1beta1::Coin};
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmos_sdk_proto::traits::{Message, MessageExt};
use cosmos_sdk_proto::Any;

use cosmwasm_std::{to_binary, Addr, Binary, CosmosMsg};
use serde::Serialize;

use crate::ContractError;

#[derive(Debug, Clone, PartialEq)]
pub enum CosmosProtoMsg {
    WithdrawDelegatorReward(MsgWithdrawDelegatorReward),
    Delegate(MsgDelegate),
    ExecuteContract(MsgExecuteContract),
    Exec(MsgExec),
}

impl TryFrom<&CosmosProtoMsg> for Any {
    fn try_from(proto: &CosmosProtoMsg) -> Result<Self, Self::Error> {
        match proto {
            CosmosProtoMsg::WithdrawDelegatorReward(msg) => msg
                .to_any()
                .map_err(|_| ContractError::GenerateExecFailure {}),
            CosmosProtoMsg::Delegate(msg) => msg
                .to_any()
                .map_err(|_| ContractError::GenerateExecFailure {}),
            CosmosProtoMsg::ExecuteContract(msg) => msg
                .to_any()
                .map_err(|_| ContractError::GenerateExecFailure {}),
            CosmosProtoMsg::Exec(msg) => Ok(Any {
                type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
                value: Binary::from(msg.encode_to_vec()).to_vec(),
            }),
        }
    }

    type Error = ContractError;
}

/// Creates a MsgExecuteContract message
pub fn create_exec_contract_msg<T, N>(
    contract_addr: String,
    sender: &N,
    msg: &T,
    funds: Option<Vec<Coin>>,
) -> Result<MsgExecuteContract, ContractError>
where
    T: Serialize + ?Sized,
    N: Into<String> + std::fmt::Display,
{
    Ok(MsgExecuteContract {
        contract: contract_addr,
        sender: sender.to_string(),
        msg: to_binary(&msg)
            .map_err(|_| ContractError::GenerateExecFailure)?
            .to_vec(),
        funds: funds.unwrap_or_default(),
    })
}

/// Creates a MsgExec message
pub fn create_exec_msg(
    grantee: &Addr,
    msgs: Vec<CosmosProtoMsg>,
) -> Result<CosmosMsg, ContractError> {
    let any_msgs: Vec<Any> = msgs
        .iter()
        .map(|msg| -> Result<Any, ContractError> { msg.try_into() })
        .collect::<Result<Vec<Any>, ContractError>>()?;

    let exec = MsgExec {
        grantee: grantee.to_string(),
        msgs: any_msgs,
    };

    Ok(CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(exec.encode_to_vec()),
    })
}
