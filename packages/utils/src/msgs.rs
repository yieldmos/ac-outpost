use cosmos_sdk_proto::cosmos::authz::v1beta1::{GenericAuthorization, Grant, MsgGrant};
use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgWithdrawDelegatorReward;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmos_sdk_proto::cosmos::{authz::v1beta1::MsgExec, base::v1beta1::Coin};
use cosmos_sdk_proto::cosmwasm::wasm::v1::MsgExecuteContract;
use cosmos_sdk_proto::prost::EncodeError;
use cosmos_sdk_proto::traits::{Message, MessageExt};
use cosmos_sdk_proto::Any;

use cosmwasm_std::{to_binary, Addr, Binary, CosmosMsg, QuerierWrapper, StdError, Uint128};
use serde::Serialize;
use wyndex::asset::{AssetInfo, AssetValidated};

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
            CosmosProtoMsg::WithdrawDelegatorReward(msg) => msg.to_any(),
            CosmosProtoMsg::Delegate(msg) => msg.to_any(),
            CosmosProtoMsg::ExecuteContract(msg) => msg.to_any(),
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

pub fn create_wyndex_swap_operations(
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
) -> wyndex_multi_hop::msg::ExecuteMsg {
    let operations = vec![wyndex_multi_hop::msg::SwapOperation::WyndexSwap {
        offer_asset_info: offer_asset.clone(),
        ask_asset_info,
    }];
    wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations {
        operations,
        minimum_receive: None,
        receiver: None,
        max_spread: None,
        referral_address: None,
        referral_commission: None,
    }
}

/// Creates a MsgExecuteContract for doing a token swap on Wyndex via the multihop router
pub fn create_wyndex_swap_msg(
    sender: &Addr,
    offer_amount: Uint128,
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
    multihop_address: String,
) -> Result<Vec<CosmosProtoMsg>, StdError> {
    // no swap to do because the offer and ask tokens are the same
    if offer_asset.eq(&ask_asset_info) {
        return Ok(vec![]);
    }

    let swap_ops = create_wyndex_swap_operations(offer_asset.clone(), ask_asset_info);

    match offer_asset {
        AssetInfo::Native(offer_denom) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            create_exec_contract_msg(
                multihop_address,
                sender,
                &swap_ops,
                Some(vec![Coin {
                    amount: offer_amount.to_string(),
                    denom: offer_denom,
                }]),
            )?,
        )]),
        AssetInfo::Token(ask_token_contract_address) => Ok(vec![CosmosProtoMsg::ExecuteContract(
            create_exec_contract_msg(
                ask_token_contract_address,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: multihop_address.to_string(),
                    amount: offer_amount,
                    msg: to_binary(&swap_ops)?,
                },
                None,
            )?,
        )]),
    }
}

/// Creates a MsgExecuteContract for doing a token swap on Wyndex via the multihop router
/// also returning the simulated resultant token amount
pub fn create_wyndex_swap_msg_with_simulation(
    querier: &QuerierWrapper,
    sender: &Addr,
    offer_amount: Uint128,
    offer_asset: AssetInfo,
    ask_asset_info: AssetInfo,
    multihop_address: String,
) -> Result<(Vec<CosmosProtoMsg>, Uint128), StdError> {
    // no swap to do because the offer and ask tokens are the same
    if offer_asset.eq(&ask_asset_info) {
        return Ok((vec![], offer_amount));
    }

    let swap_ops = create_wyndex_swap_operations(offer_asset.clone(), ask_asset_info);

    let simulated_swap: wyndex::pair::SimulationResponse;

    if let wyndex_multi_hop::msg::ExecuteMsg::ExecuteSwapOperations { operations, .. } =
        swap_ops.clone()
    {
        simulated_swap = querier.query_wasm_smart(
            multihop_address.to_string(),
            &wyndex_multi_hop::msg::QueryMsg::SimulateSwapOperations {
                offer_amount,
                operations,
                referral: false,
                referral_commission: None,
            },
        )?;
    } else {
        return Err(StdError::generic_err("Could not simulate swap operations"));
    }

    let exec: MsgExecuteContract;
    match offer_asset {
        AssetInfo::Native(offer_denom) => {
            exec = create_exec_contract_msg(
                multihop_address,
                sender,
                &swap_ops,
                Some(vec![Coin {
                    amount: offer_amount.to_string(),
                    denom: offer_denom,
                }]),
            )?;
        }
        AssetInfo::Token(ask_token_contract_address) => {
            exec = create_exec_contract_msg(
                ask_token_contract_address,
                sender,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: multihop_address.to_string(),
                    amount: offer_amount,
                    msg: to_binary(&swap_ops)?,
                },
                None,
            )?;
        }
    }
    Ok((
        vec![CosmosProtoMsg::ExecuteContract(exec)],
        simulated_swap.return_amount,
    ))
}

pub struct SwapSimResponse {
    pub swap_msgs: Vec<CosmosProtoMsg>,
    pub asset: AssetInfo,
    pub simulated_return_amount: Uint128,
}

/// Creates a MsgExecuteContract for doing multiple token swaps on Wyndex via the multihop router
/// also returning the simulated resultant token amounts
pub fn create_wyndex_swaps_with_sims(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
    offer_assets: Vec<AssetValidated>,
    ask_asset: AssetInfo,
    multihop_address: String,
) -> Result<SwapSimResponse, StdError> {
    let swaps_and_sims = offer_assets
        .into_iter()
        .map(|AssetValidated { info, amount }| {
            create_wyndex_swap_msg_with_simulation(
                querier,
                delegator_addr,
                amount,
                info.into(),
                ask_asset.clone(),
                multihop_address.to_string(),
            )
        })
        .collect::<Result<Vec<_>, StdError>>()?;

    let (swap_msgs, simulated_return_amount) = swaps_and_sims.into_iter().fold(
        (vec![], Uint128::zero()),
        |(mut swaps, mut sim_total), (swap, sim)| {
            swaps.extend(swap);
            sim_total += sim;
            (swaps, sim_total)
        },
    );

    Ok(SwapSimResponse {
        swap_msgs,
        asset: ask_asset,
        simulated_return_amount,
    })
}
