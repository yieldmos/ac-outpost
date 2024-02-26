use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdResult, Timestamp, Uint128};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    ContractExecutionAuthorizationLimit, GrantBase, GrantRequirement, RevokeRequirement,
};

use juno_destinations::comp_prefs::{wyndex_asset_info_to_terraswap_asset_info, DaoAddr, JunoDestinationProject, JunoLsd};
use juno_destinations::grants::{balance_dao_grant, gelotto_lottery_grant, wyndao_staking_grant};

use universal_destinations::grants::{native_send_token, native_staking_grant};

use wynd_helpers::wynd_swap::{simulate_wynd_pool_swap, wynd_pool_swap_grant};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::helpers::ww_market_rewards_split_grants;
use crate::msg::{CompPrefsWithAddresses, JunoWhiteWhaleMarketCompoundPrefs, QueryMsg};
use crate::{
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};

pub fn query_version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn query_authorized_compounders(deps: Deps) -> AuthorizedCompoundersResponse {
    let authorized_compound_addresses: Vec<Addr> = AUTHORIZED_ADDRS.load(deps.storage).unwrap_or_default();
    let admin: Addr = ADMIN.load(deps.storage).unwrap();
    AuthorizedCompoundersResponse {
        admin,
        authorized_compound_addresses,
    }
}

/// Queries wyndex for the amount of neta that can be received for `from_token_amount` of juno
pub fn query_juno_neta_swap(
    juno_neta_addr: &str,
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<SimulationResponse, ContractError> {
    simulate_wynd_pool_swap(
        querier,
        juno_neta_addr,
        &Asset {
            info: AssetInfo::Native("ujuno".to_string()),
            amount: from_token_amount,
        },
        "uneta".to_string(),
    )
    .map_err(ContractError::from)
}

/// Queries wyndex for the amount of wynd that can be received for `from_token_amount` of juno
pub fn query_juno_wynd_swap(
    juno_wynd_addr: &str,
    querier: &QuerierWrapper,
    from_token_amount: Uint128,
) -> Result<SimulationResponse, ContractError> {
    simulate_wynd_pool_swap(
        querier,
        juno_wynd_addr,
        &Asset {
            info: AssetInfo::Native("ujuno".to_string()),
            amount: from_token_amount,
        },
        "uwynd".to_string(),
    )
    .map_err(ContractError::from)
}

impl Grantable for QueryMsg {
    type GrantSettings = CompPrefsWithAddresses;

    fn query_grants(
        grant_structure: GrantStructure<Self::GrantSettings>,
        _current_timestamp: Timestamp,
    ) -> StdResult<Vec<GrantRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data:
                CompPrefsWithAddresses {
                    comp_prefs: _comp_prefs,
                    project_addresses,
                },
            ..
        } = grant_structure.clone();
        let split_ww_rewards_grants = ww_market_rewards_split_grants(
            GrantBase {
                granter,
                grantee: outpost_contract,
                expiration,
            },
            project_addresses,
        );

        Ok(dedupe_grant_reqs(
            [split_ww_rewards_grants, gen_comp_pref_grants(grant_structure)?].concat(),
        ))
    }

    fn query_revokes(
        grant_structure: GrantStructure<Self::GrantSettings>,
    ) -> StdResult<Vec<cw_grant_spec::grants::RevokeRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data:
                CompPrefsWithAddresses {
                    comp_prefs: _comp_prefs,
                    project_addresses,
                },
            ..
        } = grant_structure.clone();

        let split_ww_rewards_grants = ww_market_rewards_split_grants(
            GrantBase {
                granter,
                grantee: outpost_contract,
                expiration,
            },
            project_addresses,
        );

        Ok(
            dedupe_grant_reqs([split_ww_rewards_grants, gen_comp_pref_grants(grant_structure)?].concat())
                .into_iter()
                .map(|grant| -> RevokeRequirement { grant.into() })
                .collect(),
        )
    }
}

pub fn gen_comp_pref_grants(
    GrantStructure {
        granter,
        grantee,
        expiration,
        grant_contract: _grant_contract,
        grant_data:
            CompPrefsWithAddresses {
                comp_prefs: JunoWhiteWhaleMarketCompoundPrefs { comp_prefs, .. },
                project_addresses,
            },
    }: GrantStructure<CompPrefsWithAddresses>,
) -> StdResult<Vec<GrantRequirement>> {
    let grant_specs = comp_prefs.relative.iter().flat_map(|action| -> Vec<GrantRequirement> {
        let base = GrantBase {
            granter: granter.clone(),
            grantee: grantee.clone(),
            expiration,
        };

        match action.destination.clone() {
            JunoDestinationProject::Unallocated {} => vec![],
            JunoDestinationProject::JunoStaking { validator_address } => {
                [
                    vec![
                        // terraswap swap to get the juno to hand off to wyndex
                        project_addresses
                            .terraswap_routes
                            .gen_terraswap_whale_swap_grant(
                                base.clone(),
                                "ujuno".to_string(),
                                project_addresses
                                    .destination_projects
                                    .white_whale
                                    .terraswap_multihop_router
                                    .clone(),
                            )
                            .unwrap(),
                    ],
                    native_staking_grant(base, None, Some(vec![validator_address])),
                ]
                .concat()
            }

            JunoDestinationProject::DaoStaking(dao) => {
                let DaoAddr {
                    juno_wyndex_pair, cw20, ..
                } = dao.get_daos_addresses(&project_addresses.destination_projects.daos);

                let (swap_address, required_key) =
                    // use the pair if possible
                    juno_wyndex_pair.map_or((
                        project_addresses.destination_projects.wynd.multihop.to_string(),
                        "execute_swap_operations".to_string(),
                    ),|pair_add| (pair_add.to_string(), "swap".to_string()));

                vec![
                    // terraswap swap to get the juno to hand off to wyndex
                    project_addresses
                        .terraswap_routes
                        .gen_terraswap_whale_swap_grant(
                            base.clone(),
                            "ujuno".to_string(),
                            project_addresses
                                .destination_projects
                                .white_whale
                                .terraswap_multihop_router
                                .clone(),
                        )
                        .unwrap(),
                    // staking permission
                    GrantRequirement::default_contract_exec_auth(base.clone(), cw20, vec!["send"], None),
                    // swap permission
                    GrantRequirement::default_contract_exec_auth(
                        base,
                        Addr::unchecked(swap_address),
                        vec![required_key],
                        Some("ujuno"),
                    ),
                ]
            }
            JunoDestinationProject::BalanceDao {} => {
                [
                    vec![
                        // terraswap swap to get the juno to hand off balance
                        project_addresses
                            .terraswap_routes
                            .gen_terraswap_whale_swap_grant(
                                base.clone(),
                                "ujuno".to_string(),
                                project_addresses
                                    .destination_projects
                                    .white_whale
                                    .terraswap_multihop_router
                                    .clone(),
                            )
                            .unwrap(),
                    ],
                    balance_dao_grant(base, project_addresses.destination_projects.balance_dao.clone()),
                ]
                .concat()
            }
            JunoDestinationProject::GelottoLottery {
                lottery,
                lucky_phrase: _lucky_phrase,
            } => [
                vec![
                    // terraswap swap to get the juno to hand offx
                    project_addresses
                        .terraswap_routes
                        .gen_terraswap_whale_swap_grant(
                            base.clone(),
                            "ujuno".to_string(),
                            project_addresses
                                .destination_projects
                                .white_whale
                                .terraswap_multihop_router
                                .clone(),
                        )
                        .unwrap(),
                ],
                gelotto_lottery_grant(
                    base,
                    lottery.get_lottery_address(&project_addresses.destination_projects.gelotto),
                ),
            ]
            .concat(),
            JunoDestinationProject::SendTokens { denom, address } => [
                // general multihop swap
                match denom.clone() {
                    AssetInfo::Native(token_denom)
                        if token_denom.eq(&project_addresses.terraswap_routes.whale_asset.to_string()) =>
                    {
                        vec![]
                    }
                    _ => vec![project_addresses
                        .terraswap_routes
                        .gen_terraswap_whale_swap_grant(
                            base.clone(),
                            denom.to_string(),
                            project_addresses
                                .destination_projects
                                .white_whale
                                .terraswap_multihop_router
                                .clone(),
                        )
                        .unwrap()],
                },
                // send to the given user
                native_send_token(base, wyndex_asset_info_to_terraswap_asset_info(denom), address),
            ]
            .concat(),
            JunoDestinationProject::MintLsd { lsd_type } => vec![
                // terraswap swap to get the juno
                project_addresses
                    .terraswap_routes
                    .gen_terraswap_whale_swap_grant(
                        base.clone(),
                        "ujuno".to_string(),
                        project_addresses
                            .destination_projects
                            .white_whale
                            .terraswap_multihop_router
                            .clone(),
                    )
                    .unwrap(),
                GrantRequirement::default_contract_exec_auth(
                    base,
                    lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
                    vec![match lsd_type {
                        JunoLsd::StakeEasySe => "stake",
                        JunoLsd::StakeEasyB => "stake_for_bjuno",
                        JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
                    }],
                    Some("ujuno"),
                ),
            ],
            JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                vec![
                    // should wind up just being the pool permission
                    project_addresses
                        .terraswap_routes
                        .gen_terraswap_whale_swap_grant(
                            base.clone(),
                            asset.to_string(),
                            project_addresses
                                .destination_projects
                                .white_whale
                                .terraswap_multihop_router
                                .clone(),
                        )
                        .unwrap(),
                    // bonding to the market
                    GrantRequirement::default_contract_exec_auth(
                        base,
                        project_addresses.destination_projects.white_whale.market.clone(),
                        vec!["bond"],
                        Some(&asset.to_string()),
                    ),
                ]
            }
            JunoDestinationProject::WyndStaking {
                bonding_period: _bonding_period,
            } => vec![
                // convert to juno
                vec![project_addresses
                    .terraswap_routes
                    .gen_terraswap_whale_swap_grant(
                        base.clone(),
                        "ujuno".to_string(),
                        project_addresses
                            .destination_projects
                            .white_whale
                            .terraswap_multihop_router
                            .clone(),
                    )
                    .unwrap()],
                // pair swap for JUNO to WYND
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                    AssetInfo::Native("ujuno".to_string()),
                    Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
                ),
                // send wynd to the staking contract and stake the tokens
                // TODO: lock down the sending and the delegation further
                wyndao_staking_grant(base, project_addresses.destination_projects.wynd.cw20.clone()),
            ]
            .into_iter()
            .flatten()
            .collect(),
            JunoDestinationProject::RacoonBet { .. } => vec![
                project_addresses
                    .terraswap_routes
                    .gen_terraswap_whale_swap_grant(
                        base.clone(),
                        project_addresses.terraswap_routes.usdc_asset.to_string(),
                        project_addresses
                            .destination_projects
                            .white_whale
                            .terraswap_multihop_router
                            .clone(),
                    )
                    .unwrap(),
                GrantRequirement::default_contract_exec_auth(
                    base,
                    project_addresses.destination_projects.racoon_bet.game.clone(),
                    vec!["place_bet"],
                    Some(project_addresses.usdc.to_string().as_str()),
                ),
            ],
            JunoDestinationProject::TokenSwap { target_denom } => {
                vec![project_addresses
                    .terraswap_routes
                    .gen_terraswap_whale_swap_grant(
                        base.clone(),
                        target_denom.to_string(),
                        project_addresses
                            .destination_projects
                            .white_whale
                            .terraswap_multihop_router
                            .clone(),
                    )
                    .unwrap()]
            }

            JunoDestinationProject::WyndLp { .. } => vec![],
            JunoDestinationProject::SparkIbcCampaign { fund: _fund } => vec![
                project_addresses
                    .terraswap_routes
                    .gen_terraswap_whale_swap_grant(
                        base.clone(),
                        project_addresses.usdc.to_string(),
                        project_addresses
                            .destination_projects
                            .white_whale
                            .terraswap_multihop_router
                            .clone(),
                    )
                    .unwrap(),
                // funding campaign
                GrantRequirement::default_contract_exec_auth(
                    base,
                    project_addresses.destination_projects.spark_ibc.fund.clone(),
                    vec!["fund"],
                    Some(&project_addresses.usdc.to_string()),
                ),
            ],
        }
    });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
