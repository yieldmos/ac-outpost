use cosmwasm_std::{coin, Addr, Coin, Decimal, Deps, QuerierWrapper, StdResult, Timestamp, Uint128};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    AuthorizationType, ContractExecutionAuthorizationFilter, ContractExecutionAuthorizationLimit, ContractExecutionSetting,
    GrantRequirement, RevokeRequirement, StakeAuthorizationPolicy, StakeAuthorizationType, StakeAuthorizationValidators,
};

use outpost_utils::juno_comp_prefs::{DaoAddr, JunoDestinationProject, JunoLsd, RacoonBetGame};

use wynd_helpers::wynd_swap::simulate_wynd_pool_swap;
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
                match action.destination.clone() {
                    JunoDestinationProject::Unallocated {} => vec![],
                    JunoDestinationProject::JunoStaking { validator_address } => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::StakeAuthorization {
                            max_tokens: None,
                            authorization_type: StakeAuthorizationType::Delegate,
                            validators: Some(StakeAuthorizationPolicy::AllowList(StakeAuthorizationValidators {
                                address: vec![validator_address],
                            })),
                        },
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    }],
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
                            GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(cw20),
                                        limit: ContractExecutionAuthorizationLimit::default(),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec!["send".to_string()],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                            // swap permission
                            GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(swap_address),
                                        limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec![required_key],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                        ]
                    }
                    JunoDestinationProject::BalanceDao {} => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                            contract_addr: Addr::unchecked(project_addresses.destination_projects.balance_dao.clone()),
                            limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec!["swap".to_string()],
                            },
                        }]),
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    }],
                    JunoDestinationProject::GelottoLottery {
                        lottery,
                        lucky_phrase: _lucky_phrase,
                    } => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                            contract_addr: Addr::unchecked(
                                lottery.get_lottery_address(&project_addresses.destination_projects.gelotto),
                            ),
                            limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec!["sender_buy_seed".to_string()],
                            },
                        }]),
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    }],
                    JunoDestinationProject::SendTokens { denom, address } => [
                        // general multihop swap
                        match denom.clone() {
                            AssetInfo::Native(token_denom) if token_denom.eq("ujuno") => vec![],
                            _ => vec![GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(
                                            project_addresses.destination_projects.wynd.multihop.clone(),
                                        ),
                                        limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec!["execute_swap_operations".to_string()],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            }],
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
                            AssetInfo::Token(contract_addr) => GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(contract_addr),
                                        limit: ContractExecutionAuthorizationLimit::default(),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec!["send".to_string()],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                        }],
                    ]
                    .concat(),
                    JunoDestinationProject::MintLsd { lsd_type } => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                            contract_addr: Addr::unchecked(
                                lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
                            ),
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
                            GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(
                                            project_addresses
                                                .destination_projects
                                                .white_whale
                                                .terraswap_multihop_router
                                                .clone(),
                                        ),
                                        limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec!["execute_swap_operations".to_string()],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                            // bonding to the market
                            GrantRequirement::GrantSpec {
                                grant_type: AuthorizationType::ContractExecutionAuthorization(vec![
                                    ContractExecutionSetting {
                                        contract_addr: Addr::unchecked(
                                            project_addresses.destination_projects.white_whale.market.clone(),
                                        ),
                                        limit: ContractExecutionAuthorizationLimit::single_fund_limit(denom),
                                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                            keys: vec!["bond".to_string()],
                                        },
                                    },
                                ]),
                                granter: granter.clone(),
                                grantee: grantee.clone(),
                                expiration,
                            },
                        ]
                    }
                    JunoDestinationProject::WyndStaking {
                        bonding_period: _bonding_period,
                    } => vec![
                        // pair swap for JUNO to WYND
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(
                                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                                ),
                                limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["swap".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                        // send wynd to the staking contract and stake the tokens
                        // TODO: lock down the sending and the delegation further
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.cw20.clone()),
                                limit: ContractExecutionAuthorizationLimit::default(),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["send".to_string(), "delegate".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                    ],
                    JunoDestinationProject::RacoonBet { game } => vec![GrantRequirement::GrantSpec {
                        grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                            contract_addr: Addr::unchecked(project_addresses.destination_projects.racoon_bet.game.clone()),
                            limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec![match game {
                                    RacoonBetGame::Slot { .. } => "slot",
                                    RacoonBetGame::HundredSidedDice { .. } => "hundred_sided_dice",
                                }
                                .to_string()],
                            },
                        }]),
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    }],
                    JunoDestinationProject::TokenSwap {
                        target_denom: _target_denom,
                    } => vec![
                        // general multihop swap
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                                limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["execute_swap_operations".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                    ],
                    JunoDestinationProject::WyndLp {
                        contract_address,
                        bonding_period: _bonding_period,
                    } => vec![
                        // general multihop swap
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                                limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["execute_swap_operations".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                        // bonding to the pool
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(contract_address),
                                limit: ContractExecutionAuthorizationLimit::default(),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    // might need a bond key as well
                                    keys: vec!["send".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                    ],
                    JunoDestinationProject::SparkIbcCampaign { fund: _fund } => vec![
                        // general multihop swap
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                                limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["execute_swap_operations".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                        // funding campaign
                        GrantRequirement::GrantSpec {
                            grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
                                contract_addr: Addr::unchecked(
                                    project_addresses.destination_projects.spark_ibc.fund.clone(),
                                ),
                                limit: ContractExecutionAuthorizationLimit::single_fund_limit(
                                    if let AssetInfo::Native(usdc) = project_addresses.usdc.clone() {
                                        usdc
                                    } else {
                                        "".to_string()
                                    },
                                ),
                                filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                    keys: vec!["fund".to_string()],
                                },
                            }]),
                            granter: granter.clone(),
                            grantee: grantee.clone(),
                            expiration,
                        },
                    ],
                }
            });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}