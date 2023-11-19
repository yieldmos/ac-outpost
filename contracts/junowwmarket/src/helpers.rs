use cosmos_sdk_proto::cosmos::bank::v1beta1::MsgSend;
use cw20::Denom;
use cw_grant_spec::grants::{GrantBase, GrantRequirement};
use outpost_utils::{
    helpers::TaxSplitResult,
    msg_gen::{create_exec_contract_msg, CosmosProtoMsg},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdResult, Uint128, WasmMsg,
};
use terraswap_helpers::terraswap_swap::{
    create_swap_msg, create_terraswap_pool_swap_msg, create_terraswap_pool_swap_msg_with_simulation,
    create_terraswap_swap_msg_with_simulation,
};
use white_whale::{
    fee_distributor::ClaimableEpochsResponse,
    fee_distributor::ExecuteMsg as FeeDistributorExecuteMsg,
    fee_distributor::QueryMsg as FeeDistributorQueryMsg,
    pool_network::{
        asset::{Asset, AssetInfo},
        router::SwapOperation,
    },
};

use crate::{
    msg::{ContractAddrs, ExecuteMsg, TerraswapRouteAddrs},
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

pub fn ww_market_rewards_split_grants(base: GrantBase, project_addresses: ContractAddrs) -> Vec<GrantRequirement> {
    vec![
        GrantRequirement::default_contract_exec_auth(
            base.clone(),
            project_addresses.destination_projects.white_whale.rewards,
            vec!["claim"],
            None,
        ),
        GrantRequirement::GrantSpec {
            grant_type: cw_grant_spec::grants::AuthorizationType::SendAuthorization {
                spend_limit: Some(coins(u128::MAX, project_addresses.terraswap_routes.whale_asset.to_string())),
                allow_list: Some(vec![project_addresses.take_rate_addr]),
            },
            granter: base.granter,
            grantee: base.grantee,
            expiration: base.expiration,
        },
    ]
}

impl TerraswapRouteAddrs {
    pub fn get_whale_pool_addr(&self, ask_asset: &str) -> Option<Addr> {
        match ask_asset {
            _ if self.usdc_asset.to_string().eq(ask_asset) => Some(self.whale_usdc_pool.clone()),
            _ if self.ampwhale_asset.to_string().eq(ask_asset) => Some(self.whale_ampwhale_pool.clone()),
            _ if self.bonewhale_asset.to_string().eq(ask_asset) => Some(self.whale_bonewhale_pool.clone()),
            _ if self.rac_asset.to_string().eq(ask_asset) => Some(self.whale_rac_pool.clone()),

            _ => None,
        }
    }

    pub fn get_whale_swap_routes(&self, ask_asset: &str) -> Option<Vec<SwapOperation>> {
        match ask_asset {
            _ if self.juno_asset.to_string().eq(ask_asset) => Some(self.whale_to_juno_route.clone()),
            _ if self.atom_asset.to_string().eq(ask_asset) => Some(self.whale_to_atom_route.clone()),

            _ => None,
        }
    }
    pub fn gen_whale_swap_with_sim(
        &self,
        sender: &Addr,
        offer_amount: Uint128,
        ask_denom: &str,
        multihop_addr: &Addr,
        querier: &QuerierWrapper,
    ) -> Result<(CosmosProtoMsg, Asset), ContractError> {
        // TODO: if we returned an array of cosmos proto msgs instead we could retun an empty array when swapping from whale to whale
        if self.whale_asset.to_string().eq(ask_denom) {
            return Err(ContractError::TerraswapNoSwapPath {
                from: "uwhale".to_string(),
                to: "uwhale".to_string(),
            });
        }

        let offer_asset = Asset {
            info: self.whale_asset.clone(),
            amount: offer_amount,
        };

        if let Some(pool_addr) = self.get_whale_pool_addr(ask_denom) {
            // Ok(create_terraswap_pool_swap_msg(sender, offer_asset, &pool_addr)?)
            let (swap_msg, amount) =
                create_terraswap_pool_swap_msg_with_simulation(querier, sender, offer_asset, &pool_addr)?;

            Ok((
                swap_msg,
                Asset {
                    // TODO: messy as heck we should change ask_denom to just be an actual terraswap asset
                    info: AssetInfo::NativeToken {
                        denom: ask_denom.to_string(),
                    },
                    amount,
                },
            ))
        } else if let Some(swap_ops) = self.get_whale_swap_routes(ask_denom) {
            let (swap_msgs, amount) = create_terraswap_swap_msg_with_simulation(
                querier,
                sender,
                offer_amount,
                swap_ops,
                multihop_addr.to_string(),
            )?;

            Ok((
                swap_msgs.first().unwrap().clone(),
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ask_denom.to_string(),
                    },
                    amount,
                },
            ))
        } else {
            Err(ContractError::TerraswapNoSwapPath {
                from: self.whale_asset.to_string(),
                to: ask_denom.to_string(),
            })
        }
    }

    pub fn gen_whale_swap(
        &self,
        sender: &Addr,
        offer_amount: Uint128,
        ask_denom: &str,
        multihop_addr: &Addr,
    ) -> Result<CosmosProtoMsg, ContractError> {
        // TODO: if we returned an array of cosmos proto msgs instead we could retun an empty array when swapping from whale to whale
        if self.whale_asset.to_string().eq(ask_denom) {
            return Err(ContractError::TerraswapNoSwapPath {
                from: "uwhale".to_string(),
                to: "uwhale".to_string(),
            });
        }

        let offer_asset = Asset {
            info: self.whale_asset.clone(),
            amount: offer_amount,
        };

        if let Some(pool_addr) = self.get_whale_pool_addr(ask_denom) {
            Ok(create_terraswap_pool_swap_msg(sender, offer_asset, &pool_addr)?)
        } else if let Some(swap_ops) = self.get_whale_swap_routes(ask_denom) {
            Ok(create_swap_msg(sender, offer_amount, swap_ops, multihop_addr.to_string())?
                .first()
                .unwrap()
                .clone())
        } else {
            Err(ContractError::TerraswapNoSwapPath {
                from: self.whale_asset.to_string(),
                to: ask_denom.to_string(),
            })
        }
    }

    pub fn gen_terraswap_whale_swap_grant(
        &self,
        base: GrantBase,
        ask_denom: String,
        multihop_addr: Addr,
    ) -> Result<GrantRequirement, ContractError> {
        if let Some(pool_addr) = self.get_whale_pool_addr(&ask_denom) {
            Ok(GrantRequirement::default_contract_exec_auth(
                base,
                pool_addr,
                vec!["swap"],
                Some(self.whale_asset.to_string().as_str()),
            ))
        } else if self.get_whale_swap_routes(&ask_denom).is_some() {
            Ok(GrantRequirement::default_contract_exec_auth(
                base,
                multihop_addr,
                vec!["execute_swap_operations"],
                Some(self.whale_asset.to_string().as_str()),
            ))
        } else {
            Err(ContractError::TerraswapNoSwapPath {
                from: self.whale_asset.to_string(),
                to: ask_denom.to_string(),
            })
        }
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
