use cosmwasm_std::{Addr, Decimal, Deps, StdResult, Timestamp};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    GrantBase, GrantRequirement, RevokeRequirement,
};
use migaloo_destinations::comp_prefs::MigalooDestinationProject;
use migaloo_destinations::grants::furnace_grant;

use universal_destinations::grants::native_staking_grant;
use white_whale::pool_network::asset::{AssetInfo};
use withdraw_rewards_tax_grant::msg::GrantSpecData;

use crate::msg::{CompPrefsWithAddresses, MigaloostakeCompoundPrefs, QueryMsg};
use crate::{
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
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
            grant_data: CompPrefsWithAddresses {
                comp_prefs,
                project_addresses,
            },
            ..
        } = grant_structure.clone();
        let withdraw_tax_grants = withdraw_rewards_tax_grant::msg::QueryMsg::query_grants(
            GrantStructure {
                granter,
                grantee: outpost_contract,
                expiration,
                grant_contract: Addr::unchecked(project_addresses.authzpp.withdraw_tax),
                grant_data: GrantSpecData {
                    taxation_addr: Addr::unchecked(project_addresses.take_rate_addr),
                    max_fee_percentage: comp_prefs.tax_fee.unwrap_or(Decimal::MAX),
                },
            },
            current_timestamp,
        )?;

        Ok([withdraw_tax_grants, gen_comp_pref_grants(grant_structure)?].concat())
    }

    fn query_revokes(
        grant_structure: GrantStructure<Self::GrantSettings>,
    ) -> StdResult<Vec<cw_grant_spec::grants::RevokeRequirement>> {
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
        let withdraw_tax_grants = withdraw_rewards_tax_grant::msg::QueryMsg::query_revokes(GrantStructure {
            granter,
            grantee: outpost_contract,
            expiration,
            grant_contract: Addr::unchecked(project_addresses.authzpp.withdraw_tax),
            grant_data: GrantSpecData {
                taxation_addr: Addr::unchecked(project_addresses.take_rate_addr),
                max_fee_percentage: comp_prefs.tax_fee.unwrap_or(Decimal::MAX),
            },
        })?;

        Ok([
            withdraw_tax_grants,
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
                comp_prefs: MigaloostakeCompoundPrefs { comp_prefs, .. },
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
            MigalooDestinationProject::Unallocated {} => vec![],
            MigalooDestinationProject::MigalooStaking { validator_address } => {
                native_staking_grant(base, None, Some(vec![validator_address]))
            }
            MigalooDestinationProject::Furnace { and_then } => furnace_grant(
                base,
                project_addresses.destination_projects.projects.clone(),
                and_then,
                AssetInfo::NativeToken {
                    denom: project_addresses.destination_projects.denoms.ash.clone(),
                },
            ),
            _ => vec![],
            // MigalooDestinationProject::DaoStaking(dao) => {
            //     let DaoAddr {
            //         juno_wyndex_pair, cw20, ..
            //     } = dao.get_daos_addresses(&project_addresses.destination_projects.daos);

            //     let (swap_address, required_key) =
            //         // use the pair if possible
            //         juno_wyndex_pair.map_or((
            //             project_addresses.destination_projects.wynd.multihop.to_string(),
            //             "execute_swap_operations".to_string(),
            //         ),|pair_add| (pair_add.to_string(), "swap".to_string()));

            //     vec![
            //         // staking permission
            //         GrantRequirement::default_contract_exec_auth(base.clone(), cw20, vec!["send"], None),
            //         // swap permission
            //         GrantRequirement::default_contract_exec_auth(
            //             base,
            //             Addr::unchecked(swap_address),
            //             vec![required_key],
            //             Some("ujuno"),
            //         ),
            //     ]
            // }
            // MigalooDestinationProject::BalanceDao {} => {
            //     balance_dao_grant(base, project_addresses.destination_projects.balance_dao.clone())
            // }
            // MigalooDestinationProject::GelottoLottery {
            //     lottery,
            //     lucky_phrase: _lucky_phrase,
            // } => gelotto_lottery_grant(
            //     base,
            //     lottery.get_lottery_address(&project_addresses.destination_projects.gelotto),
            // ),
            // MigalooDestinationProject::SendTokens { denom, address } => [
            //     // general multihop swap
            //     match denom.clone() {
            //         AssetInfo::Native(token_denom) if token_denom.eq("ujuno") => vec![],
            //         _ => wynd_multihop_swap_grant(
            //             base.clone(),
            //             project_addresses.destination_projects.wynd.multihop.clone(),
            //             AssetInfo::Native("ujuno".to_string()),
            //             Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
            //         ),
            //     },
            //     // send to the given user
            //     vec![match denom {
            //         // if it's a native denom we need a send authorization
            //         AssetInfo::Native(denom) => GrantRequirement::GrantSpec {
            //             grant_type: AuthorizationType::SendAuthorization {
            //                 spend_limit: Some(vec![coin(u128::MAX, denom)]),
            //                 allow_list: Some(vec![Addr::unchecked(address)]),
            //             },
            //             granter: granter.clone(),
            //             grantee: grantee.clone(),
            //             expiration,
            //         },
            //         // if it's a cw20 then we need a contract execution authorization on the cw20 contract
            //         AssetInfo::Token(contract_addr) => GrantRequirement::default_contract_exec_auth(
            //             base,
            //             Addr::unchecked(contract_addr),
            //             vec!["transfer"],
            //             None,
            //         ),
            //     }],
            // ]
            // .concat(),
            // MigalooDestinationProject::MintLsd { lsd_type } => vec![GrantRequirement::default_contract_exec_auth(
            //     base,
            //     lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
            //     vec![match lsd_type {
            //         JunoLsd::StakeEasySe => "stake",
            //         JunoLsd::StakeEasyB => "stake_for_bjuno",
            //         JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
            //     }],
            //     Some("ujuno"),
            // )],
            // MigalooDestinationProject::WhiteWhaleSatellite { asset } => {
            //     let denom = match asset {
            //         AssetInfo::Native(denom) => denom,
            //         AssetInfo::Token(token) => token,
            //     };

            //     vec![
            //         // general terraswap multihop swap
            //         terraswap_multihop_swap_grant(
            //             base.clone(),
            //             project_addresses
            //                 .destination_projects
            //                 .white_whale
            //                 .terraswap_multihop_router
            //                 .clone(),
            //             "ujuno",
            //         ),
            //         // bonding to the market
            //         vec![GrantRequirement::default_contract_exec_auth(
            //             base,
            //             project_addresses.destination_projects.white_whale.market.clone(),
            //             vec!["bond"],
            //             Some(&denom),
            //         )],
            //     ]
            //     .into_iter()
            //     .flatten()
            //     .collect()
            // }
            // MigalooDestinationProject::WyndStaking {
            //     bonding_period: _bonding_period,
            // } => vec![
            //     // pair swap for JUNO to WYND
            //     wynd_pool_swap_grant(
            //         base.clone(),
            //         project_addresses.destination_projects.wynd.juno_wynd_pair.clone(),
            //         AssetInfo::Native("ujuno".to_string()),
            //         Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
            //     ),
            //     // send wynd to the staking contract and stake the tokens
            //     // TODO: lock down the sending and the delegation further
            //     wyndao_staking_grant(base, project_addresses.destination_projects.wynd.cw20.clone()),
            // ]
            // .into_iter()
            // .flatten()
            // .collect(),
            // MigalooDestinationProject::RacoonBet { .. } => vec![GrantRequirement::default_contract_exec_auth(
            //     base,
            //     project_addresses.destination_projects.racoon_bet.game.clone(),
            //     vec!["place_bet"],
            //     Some("ujuno"),
            // )],
            // MigalooDestinationProject::TokenSwap {
            //     target_denom: _target_denom,
            // } => wynd_multihop_swap_grant(
            //     base,
            //     project_addresses.destination_projects.wynd.multihop.clone(),
            //     AssetInfo::Native("ujuno".to_string()),
            //     None,
            // ),

            // MigalooDestinationProject::WyndLp { .. } => vec![
            //     // // general multihop swap
            //     // GrantRequirement::GrantSpec {
            //     //     grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
            //     //         contract_addr: project_addresses.destination_projects.wynd.multihop.clone(),
            //     //         limit: ContractExecutionAuthorizationLimit::single_fund_limit("ujuno"),
            //     //         filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
            //     //             keys: vec!["execute_swap_operations".to_string()],
            //     //         },
            //     //     }]),
            //     //     granter: granter.clone(),
            //     //     grantee: grantee.clone(),
            //     //     expiration,
            //     // },
            //     // // bonding to the pool
            //     // GrantRequirement::GrantSpec {
            //     //     grant_type: AuthorizationType::ContractExecutionAuthorization(vec![ContractExecutionSetting {
            //     //         contract_addr: Addr::unchecked(contract_address),
            //     //         limit: ContractExecutionAuthorizationLimit::default(),
            //     //         filter: ContractExecutionAuthorizationFilter::AcceptedMessageKeysFilter {
            //     //             // might need a bond key as well
            //     //             keys: vec!["send".to_string()],
            //     //         },
            //     //     }]),
            //     //     granter: granter.clone(),
            //     //     grantee: grantee.clone(),
            //     //     expiration,
            //     // },
            // ],
            // MigalooDestinationProject::SparkIbcCampaign { fund: _fund } => vec![
            //     wynd_multihop_swap_grant(
            //         base.clone(),
            //         project_addresses.destination_projects.wynd.multihop.clone(),
            //         AssetInfo::Native("ujuno".to_string()),
            //         Some(ContractExecutionAuthorizationLimit::single_fund_limit("ujuno")),
            //     ),
            //     // funding campaign
            //     vec![GrantRequirement::default_contract_exec_auth(
            //         base,
            //         project_addresses.destination_projects.spark_ibc.fund.clone(),
            //         vec!["fund"],
            //         Some(&project_addresses.usdc.to_string()),
            //     )],
            // ]
            // .into_iter()
            // .flatten()
            // .collect(),
        }
    });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
