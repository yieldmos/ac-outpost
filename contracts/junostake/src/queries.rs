use cosmwasm_std::{Addr, Decimal, Deps, QuerierWrapper, Uint128};
use cw_grant_spec::grantable_trait::{GrantStructure, Grantable};
use cw_grant_spec::grants::{
    ContractExecutionAuthorizationFilter, ContractExecutionAuthorizationLimit, GrantSpec, GrantType,
    StakeAuthorizationPolicy, StakeAuthorizationType, StakeAuthorizationValidators,
};

use outpost_utils::juno_comp_prefs::{DaoAddress, JunoDestinationProject, JunoLsd, RacoonBetGame};
use withdraw_rewards_tax_grant::msg::GrantSpecData;
use wynd_helpers::wynd_swap::simulate_wynd_pool_swap;
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::msg::{CompPrefsWithAddresses, JunostakeCompoundPrefs, QueryMsg};
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

    fn query_grants(grant_structure: GrantStructure<Self::GrantSettings>) -> Vec<GrantSpec> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data: CompPrefsWithAddresses {
                comp_prefs,
                project_addresses,
            },
            ..
        } = grant_structure.clone();
        let withdraw_tax_grants = withdraw_rewards_tax_grant::msg::QueryMsg::query_grants(GrantStructure {
            granter,
            grantee: outpost_contract,
            expiration,
            grant_contract: Addr::unchecked(project_addresses.authzpp.withdraw_tax),
            grant_data: GrantSpecData {
                max_fee_percentage: comp_prefs.tax_fee.unwrap_or(Decimal::MAX),
            },
        });

        [withdraw_tax_grants, gen_comp_pref_grants(grant_structure)].concat()
    }
}

pub fn gen_comp_pref_grants(
    GrantStructure {
        granter,
        grantee,
        expiration,
        grant_contract,
        grant_data:
            CompPrefsWithAddresses {
                comp_prefs: JunostakeCompoundPrefs { comp_prefs, .. },
                project_addresses,
            },
    }: GrantStructure<CompPrefsWithAddresses>,
) -> Vec<GrantSpec> {
    let grant_specs = comp_prefs.relative.iter().flat_map(|action| -> Vec<GrantSpec> {
        match action.destination.clone() {
            JunoDestinationProject::Unallocated {} => vec![],
            JunoDestinationProject::JunoStaking { validator_address } => vec![GrantSpec {
                grant_type: GrantType::StakeAuthorization {
                    max_tokens: None,
                    authorization_type: StakeAuthorizationType::Delegate,
                    validators: Some(StakeAuthorizationPolicy::AllowList(StakeAuthorizationValidators {
                        address: vec![validator_address.clone()],
                    })),
                },
                granter: granter.clone(),
                grantee: grantee.clone(),
                expiration,
            }],
            JunoDestinationProject::DaoStaking(dao) => {
                let DaoAddress {
                    juno_wyndex_pair,
                    staking,
                    ..
                } = dao.get_daos_addresses(&project_addresses.destination_projects.daos);

                let (swap_address, required_key) =
                    juno_wyndex_pair.map(|pair_add| (pair_add, "swap".to_string())).unwrap_or((
                        project_addresses.destination_projects.wynd.multihop.clone(),
                        "execute_swap_operations".to_string(),
                    ));

                vec![
                    // staking permission
                    GrantSpec {
                        grant_type: GrantType::ContractExecutionAuthorization {
                            contract_addr: Addr::unchecked(staking),
                            limit: ContractExecutionAuthorizationLimit::default(),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec![required_key.clone()],
                            },
                        },
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    },
                    // swap permission
                    GrantSpec {
                        grant_type: GrantType::ContractExecutionAuthorization {
                            contract_addr: Addr::unchecked(swap_address),
                            limit: ContractExecutionAuthorizationLimit::default(),
                            filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                                keys: vec![required_key],
                            },
                        },
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration,
                    },
                ]
            }
            JunoDestinationProject::BalanceDao {} => vec![GrantSpec {
                grant_type: GrantType::ContractExecutionAuthorization {
                    contract_addr: Addr::unchecked(project_addresses.destination_projects.balance_dao.clone()),
                    limit: ContractExecutionAuthorizationLimit::default(),
                    filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                        keys: vec!["swap".to_string()],
                    },
                },
                granter: granter.clone(),
                grantee: grantee.clone(),
                expiration,
            }],
            JunoDestinationProject::GelottoLottery { lottery, lucky_phrase } => unimplemented!(),
            JunoDestinationProject::SendTokens { denom: _denom, address } => vec![
                // general multihop swap
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["execute_swap_operations".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
                // send to the given user
                GrantSpec {
                    grant_type: GrantType::SendAuthorization {
                        spend_limit: None,
                        allow_list: Some(vec![Addr::unchecked(address.clone())]),
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
            ],
            JunoDestinationProject::MintLsd { lsd_type } => vec![GrantSpec {
                grant_type: GrantType::ContractExecutionAuthorization {
                    contract_addr: Addr::unchecked(
                        lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
                    ),
                    limit: ContractExecutionAuthorizationLimit::default(),
                    filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                        keys: vec![match lsd_type {
                            JunoLsd::StakeEasySe | JunoLsd::StakeEasyB => "mint",
                            JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
                        }
                        .to_string()],
                    },
                },
                granter: granter.clone(),
                grantee: grantee.clone(),
                expiration,
            }],
            JunoDestinationProject::WhiteWhaleSatellite { asset } => vec![
                // general multihop swap
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["execute_swap_operations".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
                // bonding to the market
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.white_whale.market.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["mint".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
            ],
            JunoDestinationProject::WyndStaking { bonding_period } => vec![],
            JunoDestinationProject::RacoonBet { game } => vec![GrantSpec {
                grant_type: GrantType::ContractExecutionAuthorization {
                    contract_addr: Addr::unchecked(project_addresses.destination_projects.racoon_bet.game.clone()),
                    limit: ContractExecutionAuthorizationLimit::default(),
                    filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                        keys: vec![match game {
                            RacoonBetGame::Slot { .. } => "slot",
                            RacoonBetGame::HundredSidedDice { .. } => "hundred_sided_dice",
                        }
                        .to_string()],
                    },
                },
                granter: granter.clone(),
                grantee: grantee.clone(),
                expiration,
            }],
            JunoDestinationProject::TokenSwap { target_denom } => vec![
                // general multihop swap
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["execute_swap_operations".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
            ],
            JunoDestinationProject::WyndLp {
                contract_address,
                bonding_period,
            } => vec![
                // general multihop swap
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["execute_swap_operations".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
                // bonding to the pool
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(contract_address.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            // might need a bond key as well
                            keys: vec!["send".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
            ],
            JunoDestinationProject::SparkIbcCampaign { fund } => vec![
                // general multihop swap
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.wynd.multihop.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["execute_swap_operations".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
                // funding campaign
                GrantSpec {
                    grant_type: GrantType::ContractExecutionAuthorization {
                        contract_addr: Addr::unchecked(project_addresses.destination_projects.spark_ibc.fund.clone()),
                        limit: ContractExecutionAuthorizationLimit::default(),
                        filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
                            keys: vec!["fund".to_string()],
                        },
                    },
                    granter: granter.clone(),
                    grantee: grantee.clone(),
                    expiration,
                },
            ],
        }
    });

    grant_specs.collect()
}
