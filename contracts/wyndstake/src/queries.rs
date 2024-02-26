use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdResult, Timestamp, Uint128};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{AuthorizationType, GrantBase, GrantRequirement, RevokeRequirement};

use juno_destinations::comp_prefs::{wyndex_asset_info_to_terraswap_asset_info, DaoAddr, JunoDestinationProject, JunoLsd};
use juno_destinations::grants::{balance_dao_grant, gelotto_lottery_grant, wyndao_staking_grant};
use terraswap_helpers::terraswap_swap::terraswap_multihop_grant;
use universal_destinations::grants::{native_send_token, native_staking_grant};
use wynd_helpers::wynd_swap::{simulate_wynd_pool_swap, wynd_multihop_swap_grant, wynd_pool_swap_grant};
use wyndex::{
    asset::{Asset, AssetInfo},
    pair::SimulationResponse,
};

use crate::msg::{CompPrefsWithAddresses, QueryMsg, WyndstakeCompoundPrefs};
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

    fn query_grants(grant_structure: GrantStructure<Self::GrantSettings>, _current_timestamp: Timestamp) -> StdResult<Vec<GrantRequirement>> {
        let GrantStructure {
            granter,
            expiration,
            grant_contract: outpost_contract,
            grant_data: CompPrefsWithAddresses { project_addresses, .. },
            ..
        } = grant_structure.clone();

        let taxation_grants = vec![
            GrantRequirement::default_contract_exec_auth(
                GrantBase {
                    granter: granter.clone(),
                    grantee: outpost_contract.clone(),
                    expiration,
                },
                project_addresses.destination_projects.wynd.cw20.clone(),
                vec!["transfer"],
                None,
            ),
            GrantRequirement::default_contract_exec_auth(
                GrantBase {
                    granter: granter.clone(),
                    grantee: outpost_contract.clone(),
                    expiration,
                },
                project_addresses.wynd_stake_addr.clone(),
                vec!["withdraw_rewards"],
                None,
            ),
        ];

        Ok(dedupe_grant_reqs([taxation_grants, gen_comp_pref_grants(grant_structure)?].concat()))
    }

    fn query_revokes(grant_structure: GrantStructure<Self::GrantSettings>) -> StdResult<Vec<cw_grant_spec::grants::RevokeRequirement>> {
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
                comp_prefs: WyndstakeCompoundPrefs { comp_prefs, .. },
                project_addresses,
                ..
            },
    }: GrantStructure<CompPrefsWithAddresses>,
) -> StdResult<Vec<GrantRequirement>> {
    let grant_specs = comp_prefs.relative.iter().flat_map(|action| -> Vec<GrantRequirement> {
        let base = GrantBase {
            granter: granter.clone(),
            grantee: grantee.clone(),
            expiration,
        };
        let wynd_addr = project_addresses.destination_projects.wynd.cw20.clone();
        let wynd_asset_info = AssetInfo::Token(wynd_addr.to_string());
        let _juno_asset_info = AssetInfo::Native("ujuno".to_string());

        match action.destination.clone() {
            JunoDestinationProject::Unallocated {} => vec![],
            JunoDestinationProject::JunoStaking { validator_address } => vec![
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                    wynd_asset_info,
                    None,
                ),
                native_staking_grant(base, None, Some(vec![validator_address])),
            ]
            .into_iter()
            .flatten()
            .collect(),

            JunoDestinationProject::DaoStaking(dao) => {
                let DaoAddr { cw20, .. } = dao.get_daos_addresses(&project_addresses.destination_projects.daos);

                // swap permission
                let mut grants = wynd_multihop_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.multihop.clone(),
                    wynd_asset_info.clone(),
                    None,
                );

                grants.push(
                    // staking permission
                    GrantRequirement::default_contract_exec_auth(base.clone(), cw20, vec!["send"], None),
                );

                grants
            }
            JunoDestinationProject::BalanceDao {} => [
                balance_dao_grant(base.clone(), project_addresses.destination_projects.balance_dao.clone()),
                wynd_pool_swap_grant(
                    base,
                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                    wynd_asset_info,
                    None,
                ),
            ]
            .concat(),
            JunoDestinationProject::GelottoLottery {
                lottery,
                lucky_phrase: _lucky_phrase,
            } => [
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                    wynd_asset_info.clone(),
                    None,
                ),
                gelotto_lottery_grant(base, lottery.get_lottery_address(&project_addresses.destination_projects.gelotto)),
            ]
            .concat(),
            JunoDestinationProject::SendTokens { denom, address } => [
                // general multihop swap
                match denom.clone() {
                    AssetInfo::Native(token_denom) if token_denom.eq("ujuno") => vec![],
                    _ => wynd_multihop_swap_grant(
                        base.clone(),
                        project_addresses.destination_projects.wynd.multihop.clone(),
                        wynd_asset_info.clone(),
                        None,
                    ),
                },
                // send to the given user
                native_send_token(base, wyndex_asset_info_to_terraswap_asset_info(denom), address),
            ]
            .concat(),
            JunoDestinationProject::MintLsd { lsd_type } => [
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
                    wynd_asset_info,
                    None,
                ),
                vec![GrantRequirement::default_contract_exec_auth(
                    base,
                    lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
                    vec![match lsd_type {
                        JunoLsd::StakeEasySe => "stake",
                        JunoLsd::StakeEasyB => "stake_for_bjuno",
                        JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
                    }],
                    Some("ujuno"),
                )],
            ]
            .concat(),
            JunoDestinationProject::WhiteWhaleSatellite { asset } => {
                let denom = match asset {
                    AssetInfo::Native(denom) => denom,
                    AssetInfo::Token(token) => token,
                };

                vec![
                    // the initial wyndex pool swap to usdc
                    wynd_pool_swap_grant(
                        base.clone(),
                        project_addresses.destination_projects.wynd.wynd_usdc_pair.clone(),
                        wynd_asset_info,
                        None,
                    ),
                    // general terraswap multihop swap
                    terraswap_multihop_grant(
                        base.clone(),
                        project_addresses.destination_projects.white_whale.terraswap_multihop_router.clone(),
                        white_whale::pool_network::asset::AssetInfo::NativeToken {
                            denom: project_addresses.usdc.to_string(),
                        },
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
            } =>
            // send wynd to the staking contract and stake the tokens
            // TODO: lock down the sending and the delegation further
            {
                wyndao_staking_grant(base, project_addresses.destination_projects.wynd.cw20.clone())
            }

            JunoDestinationProject::RacoonBet { .. } => [
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.wynd_usdc_pair.clone(),
                    wynd_asset_info,
                    None,
                ),
                vec![GrantRequirement::default_contract_exec_auth(
                    base,
                    project_addresses.destination_projects.racoon_bet.game.clone(),
                    vec!["place_bet"],
                    Some(project_addresses.usdc.to_string().as_str()),
                )],
            ]
            .concat(),
            JunoDestinationProject::TokenSwap { target_denom: _target_denom } => {
                wynd_multihop_swap_grant(base, project_addresses.destination_projects.wynd.multihop.clone(), wynd_asset_info, None)
            }

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
                // wynd to usdc
                wynd_pool_swap_grant(
                    base.clone(),
                    project_addresses.destination_projects.wynd.wynd_usdc_pair.clone(),
                    wynd_asset_info,
                    None,
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
