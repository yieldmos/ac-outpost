use crate::msg::{CompPrefsWithAddresses, DcaPrefs, OsmodcaCompoundPrefs, QueryMsg};
use crate::{
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
    ContractError,
};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Deps, QuerierWrapper, StdResult, Timestamp, Uint128};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{
    AuthorizationType, ContractExecutionAuthorizationLimit, GrantBase, GrantRequirement, RevokeRequirement,
};
use osmosis_destinations::comp_prefs::OsmosisDestinationProject;
use universal_destinations::grants::native_staking_grant;

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
            grant_data:
                CompPrefsWithAddresses {
                    comp_frequency,
                    comp_prefs: OsmodcaCompoundPrefs { comp_prefs, tax_fee, .. },
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
                comp_prefs: OsmodcaCompoundPrefs { comp_prefs, .. },
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
                    OsmosisDestinationProject::Unallocated {} => vec![],
                    OsmosisDestinationProject::OsmosisStaking { validator_address } => {
                        native_staking_grant(base, None, Some(vec![validator_address]))
                    }
                    _ => unimplemented!()

                    // OsmosisDestinationProject::DaoStaking(dao) => {
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

                    // OsmosisDestinationProject::SendTokens { denom, address } => [
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
                    // OsmosisDestinationProject::MintLsd { lsd_type } => vec![GrantRequirement::default_contract_exec_auth(
                    //     base,
                    //     lsd_type.get_mint_address(&project_addresses.destination_projects.juno_lsds),
                    //     vec![match lsd_type {
                    //         JunoLsd::StakeEasySe => "stake",
                    //         JunoLsd::StakeEasyB => "stake_for_bjuno",
                    //         JunoLsd::Wynd | JunoLsd::Backbone | JunoLsd::Eris => "bond",
                    //     }],
                    //     Some("ujuno"),
                    // )],

                    // OsmosisDestinationProject::TokenSwap {
                    //     target_denom: _target_denom,
                    // } => wynd_multihop_swap_grant(
                    //     base,
                    //     project_addresses.destination_projects.wynd.multihop.clone(),
                    //     AssetInfo::Native("ujuno".to_string()),
                    //     None,
                    // ),
                }
            });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
