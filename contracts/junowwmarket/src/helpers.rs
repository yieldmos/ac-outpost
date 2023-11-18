use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use outpost_utils::{
    helpers::TaxSplitResult,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdResult, Uint128, WasmMsg};
use terraswap_helpers::terraswap_swap::{
    create_swap_msg, create_terraswap_pool_swap_msg, create_terraswap_pool_swap_msg_with_simulation,
    create_terraswap_swap_msg_with_simulation,
};
use white_whale::{
    fee_distributor::ClaimableEpochsResponse,
    fee_distributor::ExecuteMsg as FeeDistributorExecuteMsg,
    fee_distributor::QueryMsg as FeeDistributorQueryMsg,
    pool_network::asset::{Asset, AssetInfo},
};

use crate::{
    msg::{ExecuteMsg, TerraswapRouteAddrs},
    ContractError,
};

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

pub fn query_and_generate_ww_market_reward_msgs(
    tax_percent: Decimal,
    user_addr: &Addr,
    tax_addr: &Addr,
    ww_market_addr: &Addr,
    querier: &QuerierWrapper,
) -> Result<TaxSplitResult, ContractError> {
    let rewards: ClaimableEpochsResponse = querier.query_wasm_smart(
        ww_market_addr,
        &to_json_binary(&FeeDistributorQueryMsg::Claimable {
            address: user_addr.to_string(),
        })?,
    )?;

    let claim_and_tax_msgs = vec![
        CosmosProtoMsg::ExecuteContract(create_exec_contract_msg(
            ww_market_addr.to_string(),
            &user_addr.clone(),
            &FeeDistributorExecuteMsg::Claim {},
            None,
        )?),
        CosmosProtoMsg::Send(MsgSend {
            from_address: user_addr.to_string(),
            to_address: tax_addr.to_string(),
            amount: vec![
            //     cosmos_sdk_proto::cosmos::base::v1beta1::Coin {
            //     denom: token.denom.clone(),
            //     amount: tax_amount.to_string(),
            // }
            ],
        }),
    ];

    Ok(TaxSplitResult {
        remaining_rewards: todo!(),
        tax_amount: todo!(),
        claim_and_tax_msgs,
    })
}

impl TerraswapRouteAddrs {
    pub fn gen_whale_swap_with_sim(
        &self,
        sender: &Addr,
        offer_amount: Uint128,
        ask_denom: &str,
        multihop_addr: &Addr,
        querier: &QuerierWrapper,
    ) -> Result<(CosmosProtoMsg, Asset), ContractError> {
        let offer_asset = Asset {
            info: self.whale_asset.clone(),
            amount: offer_amount,
        };
        Ok(match ask_denom {
            // we have pools for the first few assets
            denom if self.usdc_asset_info.to_string().eq(denom) => {
                let (swap_msg, amount) =
                    create_terraswap_pool_swap_msg_with_simulation(querier, sender, offer_asset, &self.whale_usdc_pool)?;

                (
                    swap_msg,
                    Asset {
                        info: self.usdc_asset_info.clone(),
                        amount,
                    },
                )
            }
            denom if self.ampwhale_asset_info.to_string().eq(denom) => {
                let (swap_msg, amount) =
                    create_terraswap_pool_swap_msg_with_simulation(querier, sender, offer_asset, &self.whale_ampwhale_pool)?;

                (
                    swap_msg,
                    Asset {
                        info: self.ampwhale_asset_info.clone(),
                        amount,
                    },
                )
            }
            denom if self.bonewhale_asset_info.to_string().eq(denom) => {
                let (swap_msg, amount) = create_terraswap_pool_swap_msg_with_simulation(
                    querier,
                    sender,
                    offer_asset,
                    &self.whale_bonewhale_pool,
                )?;

                (
                    swap_msg,
                    Asset {
                        info: self.bonewhale_asset_info.clone(),
                        amount,
                    },
                )
            }

            // juno should be a common target but we dont have a pool for it and it requires a multi-hop
            denom if denom.eq("ujuno") => {
                let (swap_msgs, amount) = create_terraswap_swap_msg_with_simulation(
                    querier,
                    sender,
                    offer_amount,
                    self.whale_to_juno_route.clone(),
                    multihop_addr.to_string(),
                )?;

                (
                    swap_msgs.first().unwrap().clone(),
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "ujuno".to_string(),
                        },
                        amount,
                    },
                )
            }

            // if the ask denom is outside of these four we should bail since we have no dynamic pathfinding
            _ => Err(ContractError::TerraswapNoSwapPath {
                from: self.whale_asset.to_string(),
                to: ask_denom.to_string(),
            })?,
        })
    }
    pub fn gen_whale_swap(
        &self,
        sender: &Addr,
        offer_amount: Uint128,
        ask_denom: &str,
        multihop_addr: &Addr,
    ) -> Result<CosmosProtoMsg, ContractError> {
        let offer_asset = Asset {
            info: self.whale_asset.clone(),
            amount: offer_amount,
        };
        Ok(match ask_denom {
            // we have pools for the first few assets
            denom if self.usdc_asset_info.to_string().eq(denom) => {
                create_terraswap_pool_swap_msg(sender, offer_asset, &self.whale_usdc_pool)?
            }
            denom if self.ampwhale_asset_info.to_string().eq(denom) => {
                create_terraswap_pool_swap_msg(sender, offer_asset, &self.whale_ampwhale_pool)?
            }
            denom if self.bonewhale_asset_info.to_string().eq(denom) => {
                create_terraswap_pool_swap_msg(sender, offer_asset, &self.whale_bonewhale_pool)?
            }

            // juno should be a common target but we dont have a pool for it and it requires a multi-hop
            denom if denom.eq("ujuno") => create_swap_msg(
                sender,
                offer_amount,
                self.whale_to_juno_route.clone(),
                multihop_addr.to_string(),
            )?
            .first()
            .unwrap()
            .clone(),

            // if the ask denom is outside of these four we should bail since we have no dynamic pathfinding
            _ => Err(ContractError::TerraswapNoSwapPath {
                from: self.whale_asset.to_string(),
                to: ask_denom.to_string(),
            })?,
        })
    }
}

/// converts from a Wyndex AssetInfo to a Terraswap AssetInfo
pub fn wyndex_assetinfo_to_terraswap_assetinfo(
    asset_info: wyndex::asset::AssetInfo,
) -> white_whale::pool_network::asset::AssetInfo {
    match asset_info {
        wyndex::asset::AssetInfo::Native(denom) => AssetInfo::NativeToken { denom },
        wyndex::asset::AssetInfo::Token(contract_addr) => AssetInfo::Token { contract_addr },
    }
}

/// fn to convert from a terraswap asset info to a wyndex asset info
pub fn terraswap_assetinfo_to_wyndex_assetinfo(
    asset_info: white_whale::pool_network::asset::AssetInfo,
) -> wyndex::asset::AssetInfo {
    match asset_info {
        AssetInfo::NativeToken { denom } => wyndex::asset::AssetInfo::Native(denom),
        AssetInfo::Token { contract_addr } => wyndex::asset::AssetInfo::Token(contract_addr),
    }
}

/// converts a terraswap asset into a coin or errors (if it's a cw20)
pub fn asset_to_coin(asset: Asset) -> Result<Coin, ContractError> {
    match asset.info {
        AssetInfo::NativeToken { denom } => Ok(Coin {
            denom,
            amount: asset.amount,
        }),
        AssetInfo::Token { contract_addr } => Err(ContractError::AssetIsNotCoinable(contract_addr)),
    }
}
