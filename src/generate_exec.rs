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

pub fn create_exec_contract_msg<T, N>(
    contract_addr: &String,
    sender: &N,
    msg: &T,
    funds: Option<Vec<Coin>>,
) -> Result<MsgExecuteContract, ContractError>
where
    T: Serialize + ?Sized,
    N: Into<String> + std::fmt::Display,
{
    Ok(MsgExecuteContract {
        contract: contract_addr.to_string(),
        sender: sender.to_string(),
        msg: to_binary(&msg)
            .map_err(|_| ContractError::GenerateExecFailure)?
            .to_vec(),
        funds: funds.unwrap_or(vec![]),
    })
}

pub fn create_exec_msg(grantee: &Addr, msgs: &Vec<Any>) -> CosmosMsg {
    let exec = MsgExec {
        grantee: grantee.to_string(),
        msgs: msgs.to_vec(),
    };

    CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(exec.encode_to_vec()),
    }
}

// pub fn create_exec_msg<T>(grantee: &Addr, msgs: &Vec<T>) -> Result<CosmosMsg, ContractError>
// where
//     T: MessageExt + TypeUrl,
// {
//     let any_msgs = msgs
//         .into_iter()
//         .map(|msg| {
//             msg.to_any()
//                 .map_err(|_| ContractError::GenerateExecFailure {})
//         })
//         .collect::<Result<Vec<_>, _>>()?;

//     let exec = MsgExec {
//         grantee: grantee.to_string(),
//         msgs: any_msgs,
//     };

//     Ok(CosmosMsg::Stargate {
//         type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
//         value: Binary::from(exec.encode_to_vec()),
//     })

//     // // let example_msg = MsgExecuteContract {
//     // //     contract: "testcontractAddr".to_string(),
//     // //     sender: "testwallet".to_string(),
//     // //     msg: to_binary(&BowStakingExecuteMsg::Stake {}).unwrap().to_vec(),
//     // //     funds: vec![amount_to_stake.clone()],
//     // // }

//     // let exec_msg = CosmosMsg::Stargate {
//     //     type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
//     //     value: Binary(
//     //         MsgExec {
//     //             grantee: grantee.to_string(),
//     //             msgs: vec![Any {
//     //                 type_url: "/cosmwasm.wasm.v1.MsgExecuteContract".to_string(),
//     //                 value: buffer,
//     //             }],
//     //         }
//     //         .encode_to_vec(),
//     //     ),
//     // };

//     // let mut coin = Coin::new();

//     // coin.denom = amount.denom.to_string();
//     // coin.amount = amount.amount.to_string();

//     // let mut delegation_msg = MsgDelegate::new();

//     // delegation_msg.delegator_address = granter.to_string();
//     // delegation_msg.validator_address = valaddress.to_string();
//     // delegation_msg.amount = MessageField::some(coin);

//     // let mut any = Any::new();
//     // any.type_url = "/cosmos.staking.v1beta1.MsgDelegate".to_string();
//     // any.value = delegation_msg.write_to_bytes().unwrap();
//     // let mut messages: Vec<Any> = Vec::new();
//     // messages.push(any);

//     //If the granter didn't grant this contract permission to delegate in his place, this execution will fail.
// }
