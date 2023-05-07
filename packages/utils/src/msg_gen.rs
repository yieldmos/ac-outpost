use cosmos_sdk_proto::cosmos::authz::v1beta1::{GenericAuthorization, Grant, MsgGrant};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgWithdrawDelegatorReward;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::{authz::v1beta1::MsgExec, base::v1beta1::Coin};
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmos_sdk_proto::prost::EncodeError;
use cosmos_sdk_proto::traits::{Message, MessageExt, TypeUrl};
use cosmos_sdk_proto::Any;

use cosmwasm_std::{to_binary, Addr, Binary, CosmosMsg, StdError};

use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountOut,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub enum CosmosProtoMsg {
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
            CosmosProtoMsg::WithdrawDelegatorReward(msg) => msg.to_any(),
            CosmosProtoMsg::Delegate(msg) => msg.to_any(),
            CosmosProtoMsg::ExecuteContract(msg) => msg.to_any(),
            CosmosProtoMsg::OsmosisSwapExactAmountIn(msg) => {
                let any = Any::default();

                // let b: CosmosMsg = *msg.into::cosmwasm_std::CosmosMsg()?;
                // MsgSwapExactAmountIn::encode(&self, buf)
                // MsgSwapExactAmountIn::TYPE_URL;
                // MsgSwapExactAmountIn::encode(msg, &mut any.value)?;
                todo!("need to get the conversion to any working correctly");
                Ok(Any::default())
            }
            CosmosProtoMsg::OsmosisSwapExactAmountOut(msg) => unimplemented!(),
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
    contract_addr: String,
    sender: &N,
    msg: &T,
    funds: Option<Vec<Coin>>,
) -> Result<MsgExecuteContract, StdError>
where
    T: Serialize + ?Sized,
    N: Into<String> + std::fmt::Display,
{
    Ok(MsgExecuteContract {
        contract: contract_addr,
        sender: sender.to_string(),
        msg: to_binary(&msg)?.to_vec(),
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
