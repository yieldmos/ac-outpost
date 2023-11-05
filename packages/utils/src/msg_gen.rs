use cosmos_sdk_proto::cosmos::authz::v1beta1::{GenericAuthorization, Grant, MsgGrant};
use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgWithdrawDelegatorReward;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::{authz::v1beta1::MsgExec, base::v1beta1::Coin};
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmos_sdk_proto::prost::EncodeError;
use cosmos_sdk_proto::traits::{Message, MessageExt};
use cosmos_sdk_proto::Any;

use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, StdError};

use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountOut,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub enum CosmosProtoMsg {
    Send(MsgSend),
    WithdrawDelegatorReward(MsgWithdrawDelegatorReward),
    Delegate(MsgDelegate),
    ExecuteContract(MsgExecuteContract),
    Exec(MsgExec),
    OsmosisSwapExactAmountIn(MsgSwapExactAmountIn),
    OsmosisSwapExactAmountOut(MsgSwapExactAmountOut),
}

impl TryFrom<&CosmosProtoMsg> for Any {
    fn try_from(proto: &CosmosProtoMsg) -> Result<Self, Self::Error> {
        match proto {
            CosmosProtoMsg::Send(msg) => msg.to_any(),
            CosmosProtoMsg::WithdrawDelegatorReward(msg) => msg.to_any(),
            CosmosProtoMsg::Delegate(msg) => msg.to_any(),
            CosmosProtoMsg::ExecuteContract(msg) => msg.to_any(),
            CosmosProtoMsg::OsmosisSwapExactAmountIn(msg) => Ok(Any {
                type_url: MsgSwapExactAmountIn::TYPE_URL.to_string(),
                value: msg.clone().encode_to_vec(),
            }),
            CosmosProtoMsg::OsmosisSwapExactAmountOut(msg) => Ok(Any {
                type_url: MsgSwapExactAmountOut::TYPE_URL.to_string(),
                value: msg.clone().encode_to_vec(),
            }),
            CosmosProtoMsg::Exec(msg) => Ok(Any {
                type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
                value: Binary::from(msg.encode_to_vec()).to_vec(),
            }),
        }
    }

    type Error = EncodeError;
}

/// Creates a MsgExecuteContract message
pub fn create_exec_contract_msg<T, N>(
    contract_addr: impl Into<String>,
    sender: &N,
    msg: &T,
    funds: Option<Vec<Coin>>,
) -> Result<MsgExecuteContract, StdError>
where
    T: Serialize + ?Sized,
    N: Into<String> + std::fmt::Display,
{
    Ok(MsgExecuteContract {
        contract: contract_addr.into(),
        sender: sender.to_string(),
        msg: to_json_binary(&msg)?.to_vec(),
        funds: funds.unwrap_or_default(),
    })
}

pub enum GenericAuthorizationType {
    WithdrawDelegatorRewards,
    Delegation,
}

impl From<GenericAuthorizationType> for Any {
    fn from(proto: GenericAuthorizationType) -> Any {
        match proto {
            GenericAuthorizationType::WithdrawDelegatorRewards => Any {
                type_url: "/cosmos.authz.v1beta1.GenericAuthorization".to_string(),
                value: GenericAuthorization {
                    msg: "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward".to_string(),
                }
                .encode_to_vec(),
            },
            GenericAuthorizationType::Delegation => Any {
                type_url: "/cosmos.authz.v1beta1.GenericAuthorization".to_string(),
                value: GenericAuthorization {
                    msg: "/cosmos.staking.v1beta1.MsgDelegate".to_string(),
                }
                .encode_to_vec(),
            },
        }
    }
}

/// Creates a Generic MsgGrant message
pub fn create_generic_grant_msg(
    granter: String,
    grantee: &Addr,
    grant_type: GenericAuthorizationType,
) -> CosmosMsg {
    let grant = MsgGrant {
        grantee: grantee.to_string(),
        granter,
        grant: Some(Grant {
            authorization: Some(grant_type.into()),
            expiration: None,
        }),
    };

    CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgGrant".to_string(),
        value: Binary::from(grant.encode_to_vec()),
    }
}

/// Creates a MsgExec message
pub fn create_exec_msg(
    grantee: &Addr,
    msgs: Vec<CosmosProtoMsg>,
) -> Result<CosmosMsg, EncodeError> {
    let any_msgs: Vec<Any> = msgs
        .iter()
        .map(|msg| -> Result<Any, EncodeError> { msg.try_into() })
        .collect::<Result<Vec<Any>, EncodeError>>()?;

    let exec = MsgExec {
        grantee: grantee.to_string(),
        msgs: any_msgs,
    };

    Ok(CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(exec.encode_to_vec()),
    })
}
