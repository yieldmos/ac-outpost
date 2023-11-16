use cosmwasm_std::{coin, Addr, Coin, Decimal, Deps, QuerierWrapper, StdResult, Timestamp, Uint128};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    AuthorizationType, ContractExecutionAuthorizationFilter, ContractExecutionAuthorizationLimit, ContractExecutionSetting,
    GrantBase, GrantRequirement, RevokeRequirement,
};

use juno_helpers::grants::{balance_dao_grant, gelotto_lottery_grant, native_staking_grant, wyndao_staking_grant};
use outpost_utils::juno_comp_prefs::{DaoAddr, JunoDestinationProject, JunoLsd};

use terraswap_helpers::terraswap_swap::terraswap_multihop_swap_grant;
use wynd_helpers::wynd_swap::{simulate_wynd_pool_swap, wynd_multihop_swap_grant, wynd_pool_swap_grant};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::msg::{CompPrefsWithAddresses, DcaPrefs, JunodcaCompoundPrefs, QueryMsg};
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
        current_timestamp: Timestamp,
    ) -> StdResult<Vec<GrantRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data:
                CompPrefsWithAddresses {
                    comp_frequency,
                    comp_prefs: JunodcaCompoundPrefs { comp_prefs, tax_fee, .. },
                    project_addresses,
                },
            ..
        } = grant_structure.clone();

        let iteration_count: Uint128 = comp_frequency.iteration_count(current_timestamp, expiration).into();
        let fee = tax_fee.unwrap_or(Decimal::percent(1));

        let taxation_grants = vec![GrantRequirement::GrantSpec {
            grant_type: AuthorizationType::SendAuthorization {
                spend_limit: Some(
                    comp_prefs
                        .into_iter()
                        // get the compounding tokens
                        .map(|DcaPrefs { compound_token, .. }| compound_token)
                        // estimate the amount of tokens that will be received
                        .map(|Coin { amount, denom }| Coin {
                            amount: (amount * iteration_count) * fee,
                            denom,
                        })
                        .collect(),
                ),
                allow_list: Some(vec![project_addresses.take_rate_addr.clone()]),
            },
            granter,
            grantee: outpost_contract,
            expiration,
        }];

        Ok(dedupe_grant_reqs(
            [taxation_grants, gen_comp_pref_grants(grant_structure)?].concat(),
        ))
    }

    fn query_revokes(
        grant_structure: GrantStructure<Self::GrantSettings>,
    ) -> StdResult<Vec<cw_grant_spec::grants::RevokeRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            ..
        } = grant_structure.clone();
        let taxation_revoke: Vec<RevokeRequirement> = vec![GrantRequirement::GrantSpec {
            grant_type: AuthorizationType::SendAuthorization {
                spend_limit: None,
                allow_list: None,
            },
            granter,
            grantee: outpost_contract,
            expiration,
        }
        .into()];

        Ok([
            taxation_revoke,
            gen_comp_pref_grants(grant_structure)?
                .into_iter()
                .map(|grant| -> RevokeRequirement { grant.into() })
                .collect(),
        ]
        .concat())
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
                comp_prefs: JunodcaCompoundPrefs { comp_prefs, .. },
                project_addresses,
                ..
            },
    }: GrantStructure<CompPrefsWithAddresses>,
) -> StdResult<Vec<GrantRequirement>> {
    let grant_specs =
        comp_prefs
            .first()
            .unwrap()
            .compound_preferences
            .relative
            .iter()
            .flat_map(|action| -> Vec<GrantRequirement> {
                let base = GrantBase {
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                };

                match action.destination.clone() {
                    JunoDestinationProject::Unallocated {} => vec![],
                    JunoDestinationProject::JunoStaking { validator_address } => {
                        native_staking_grant(base, None, Some(vec![validator_address]))
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
                        balance_dao_grant(base, project_addresses.destination_projects.balance_dao.clone())
                    }
                    JunoDestinationProject::GelottoLottery {
                        lottery,
                        lucky_phrase: _lucky_phrase,
                    } => gelotto_lottery_grant(
                        base,
                        lottery.get_lottery_address(&project_addresses.destination_projects.gelotto),
                    ),
                    JunoDestinationProject::SendTokens { denom, address } => [
                        // general multihop swap
                        match denom.clone() {
                            AssetInfo::Native(token_denom) if token_denom.eq("ujuno") => vec![],
                            _ => wynd_multihop_swap_grant(
                                base.clone(),
                                project_addresses.destination_projects.wynd.multihop.clone(),
                                AssetInfo::Native("ujuno".to_string()),
                                Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
                            ),
                        },
                        // send to the given user
                        vec![match denom {
                            // if it's a native denom we need a send authorization
                            AssetInfo::Native(denom) => GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::SendAuthorization {
                                    spend_limit: Some(vec![coin(u128::MAX, denom)]),
                                    allow_list: Some(vec![Addr::unchecked(address)]),
                                },
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                            // if it's a cw20 then we need a contract execution authorization on the cw20 contract
                            AssetInfo::Token(contract_addr) => GrantRequirement::default_contract_exec_auth(
                                base,
                                Addr::unchecked(contract_addr),
                                vec!["send"],
                                None,
                            ),
                        }],
                    ]
                    .concat(),
                    JunoDestinationProject::MintLsd { lsd_type } => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                            contract_addr: lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),

                            limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec![match lsd_type {
                                    JunoLsd::StakeEasySe => "stake",
                                    JunoLsd::StakeEasyB => "stake_for_bjuno",
                                    JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
                                }
                                .to_string()],
                            },
                        }]),
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    }],
                    JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                        let denom = match asset {
                            AssetInfo::Native(denom) => denom,
                            AssetInfo::Token(token) => token,
                        };

                        vec![
                            // general terraswap multihop swap
                            terraswap_multihop_swap_grant(
                                base.clone(),
                                project_addresses
                                    .destination_projects
                                    .white_whale
                                    .terraswap_multihop_router
                                    .clone(),
                                "ujuno",
                            ),
                            // bonding to the market
                            vec![GrantRequirement::default_contract_exec_auth(
                                base,
                                project_addresses.destination_projects.white_whale.market.clone(),
                                vec!["bond"],
                                Some(&denom),
                            )],
                        ]
                        .into_iter()
                        .flatten()
                        .collect()
                    }
                    JunoDestinationProject::WyndStaking {
                        bonding_period: _bonding_period,
                    } => vec![
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
                    JunoDestinationProject::RacoonBet { .. } => vec![GrantRequirement::default_contract_exec_auth(
                        base,
                        project_addresses.destination_projects.racoon_bet.game.clone(),
                        vec!["place_bet"],
                        Some("ujuno"),
                    )],
                    JunoDestinationProject::TokenSwap {
                        target_denom: _target_denom,
                    } => wynd_multihop_swap_grant(
                        base,
                        project_addresses.destination_projects.wynd.multihop.clone(),
                        AssetInfo::Native("ujuno".to_string()),
                        None,
                    ),

                    JunoDestinationProject::WyndLp { .. } => vec![
                        // // general multihop swap
                        // GrantRequirement::GrantSpec {
                        //     grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                        //         contract_addr: project_addresses.destination_projects.wynd.multihop.clone(),
                        //         limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                        //         filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                        //             keys: vec!["execute_swap_operations".to_string()],
                        //         },
                        //     }]),
                        //     granter: granter.clone(),
                        //     grantee: grantee.clone(),
                        //     expiration,
                        // },
                        // // bonding to the pool
                        // GrantRequirement::GrantSpec {
                        //     grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                        //         contract_addr: Addr::unchecked(contract_address),
                        //         limit: ContractExecutionAuthorizationLimit::default(),
                        //         filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                        //             // might need a bond key as well
                        //             keys: vec!["send".to_string()],
                        //         },
                        //     }]),
                        //     granter: granter.clone(),
                        //     grantee: grantee.clone(),
                        //     expiration,
                        // },
                    ],
                    JunoDestinationProject::SparkIbcCampaign { fund: _fund } => vec![
                        wynd_multihop_swap_grant(
                            base.clone(),
                            project_addresses.destination_projects.wynd.multihop.clone(),
                            AssetInfo::Native("ujuno".to_string()),
                            Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
                        ),
                        // funding campaign
                        vec![GrantRequirement::default_contract_exec_auth(
                            base,
                            project_addresses.destination_projects.spark_ibc.fund.clone(),
                            vec!["fund"],
                            Some(&project_addresses.usdc.to_string()),
                        )],
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                }
            });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
