use crate::msg::{CompPrefsWithAddresses, OsmostakeCompoundPrefs, QueryMsg};
use crate::{
    msg::{AuthorizedCompoundersResponse, VersionResponse},
    state::{ADMIN, AUTHORIZED_ADDRS},
};
use cosmwasm_std::{Addr, Deps, StdResult, Timestamp};
use cw_grant_spec::grantable_trait::{dedupe_grant_reqs, GrantStructure, Grantable};
use cw_grant_spec::grants::{GrantBase, GrantRequirement, RevokeRequirement};
use membrane_helpers::grants::{
    membrane_deposit_grant, membrane_deposit_into_stability_pool_grant, membrane_mint_cdt_grant, membrane_repay_cdt_grant,
    membrane_stake_grant,
};
use osmosis_destinations::comp_prefs::{
    MembraneDepositCollateralAction, OsmosisDepositCollateral, OsmosisDestinationProject, OsmosisLsd, OsmosisPoolSettings,
    OsmosisRepayDebt, RepayThreshold,
};
use osmosis_destinations::grants::{mint_milk_tia_grant, stake_ion_grants};
use osmosis_helpers::osmosis_lp::{join_cl_pool_grants, join_classic_pool_grants, join_osmosis_pool_grants};
use osmosis_helpers::osmosis_swap::osmosis_swap_grants;
use outpost_utils::comp_prefs::{CompoundPrefs, DestinationAction};
use sail_destinations::grants::eris_lsd_grant;
use universal_destinations::grants::{native_send_token, native_staking_grant};
use white_whale::pool_network::asset::AssetInfo;
use withdraw_rewards_tax_grant::msg::GrantSpecData;

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
                    comp_prefs: OsmostakeCompoundPrefs { comp_prefs: _, .. },
                    project_addresses,
                    take_rate,
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
                    taxation_addr: take_rate.take_rate_addr,
                    max_fee_percentage: take_rate.max_tax_fee,
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
            grant_data:
                CompPrefsWithAddresses {
                    take_rate,
                    project_addresses,
                    ..
                },
            ..
        } = grant_structure.clone();

        let withdraw_tax_revokes = withdraw_rewards_tax_grant::msg::QueryMsg::query_revokes(GrantStructure {
            granter,
            grantee: outpost_contract,
            expiration,
            grant_contract: Addr::unchecked(project_addresses.authzpp.withdraw_tax),
            grant_data: GrantSpecData {
                taxation_addr: Addr::unchecked(take_rate.take_rate_addr),
                max_fee_percentage: take_rate.max_tax_fee,
            },
        })?;

        Ok([
            withdraw_tax_revokes,
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
        grant_contract,
        grant_data:
            CompPrefsWithAddresses {
                comp_prefs:
                    OsmostakeCompoundPrefs {
                        comp_prefs,
                        user_address,
                        tax_fee,
                    },
                project_addresses,
                take_rate,
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
            OsmosisDestinationProject::Unallocated {} => vec![],
            OsmosisDestinationProject::OsmosisStaking { validator_address } => {
                native_staking_grant(base, None, Some(vec![validator_address]))
            }
            OsmosisDestinationProject::TokenSwap { target_asset: _ } => osmosis_swap_grants(base),
            OsmosisDestinationProject::SendTokens { address, target_asset } => vec![
                osmosis_swap_grants(base.clone()),
                native_send_token(
                    base,
                    AssetInfo::NativeToken {
                        denom: target_asset.denom,
                    },
                    address,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::MintLsd { lsd: OsmosisLsd::Eris } => eris_lsd_grant(
                base,
                project_addresses.destination_projects.projects.eris_amposmo_bonding.clone(),
                AssetInfo::NativeToken {
                    denom: "uosmo".to_string(),
                },
            ),
            OsmosisDestinationProject::MintLsd {
                lsd: OsmosisLsd::MilkyWay,
            } => vec![
                osmosis_swap_grants(base.clone()),
                mint_milk_tia_grant(
                    base,
                    project_addresses.destination_projects.projects.milky_way_bonding.clone(),
                    &project_addresses.destination_projects.denoms.tia,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::IonStaking {} => vec![
                osmosis_swap_grants(base.clone()),
                stake_ion_grants(
                    base,
                    project_addresses.destination_projects.projects.ion_dao.clone(),
                    &project_addresses.destination_projects.denoms.ion,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::MembraneStake {} => vec![
                osmosis_swap_grants(base.clone()),
                membrane_stake_grant(
                    base.clone(),
                    project_addresses.destination_projects.projects.membrane.staking.clone(),
                    &project_addresses.destination_projects.denoms.mbrn,
                ),
            ]
            .concat(),

            OsmosisDestinationProject::OsmosisLiquidityPool {
                pool_id: _,
                pool_settings: OsmosisPoolSettings::Standard { bond_tokens },
            } => join_classic_pool_grants(base, bond_tokens),
            OsmosisDestinationProject::OsmosisLiquidityPool {
                pool_id: _,
                pool_settings: OsmosisPoolSettings::ConcentratedLiquidity { .. },
            } => join_cl_pool_grants(base),
            OsmosisDestinationProject::DepositCollateral {
                as_asset,
                protocol: OsmosisDepositCollateral::Membrane { position_id, and_then },
            } => {
                // permission for the initial deposit into the CDP
                let deposit_grant = membrane_deposit_grant(
                    base.clone(),
                    project_addresses.destination_projects.projects.membrane.cdp.clone(),
                    position_id,
                    vec![&as_asset],
                );

                // permission for whatever the follow-up action may be requested
                let and_then_grant = match and_then {
                    Some(MembraneDepositCollateralAction::MintCdt { desired_ltv }) => membrane_mint_cdt_grant(
                        base,
                        project_addresses.destination_projects.projects.membrane.cdp.clone(),
                        position_id.clone(),
                        desired_ltv,
                    ),
                    Some(MembraneDepositCollateralAction::EnterStabilityPool { .. }) => {
                        membrane_deposit_into_stability_pool_grant(
                            base,
                            project_addresses
                                .destination_projects
                                .projects
                                .membrane
                                .stability_pool
                                .clone(),
                            &project_addresses.destination_projects.denoms.cdt,
                        )
                    }
                    Some(MembraneDepositCollateralAction::ProvideLiquidity { pool_settings, .. }) => {
                        join_osmosis_pool_grants(base, pool_settings)
                    }

                    None => vec![],
                };

                [deposit_grant, and_then_grant].concat()
            }
            OsmosisDestinationProject::RepayDebt {
                ltv_ratio_threshold,
                protocol,
            } => {
                // permission for the initial repay
                let repay_grants = match protocol {
                    OsmosisRepayDebt::Membrane { position_id } => membrane_repay_cdt_grant(
                        base.clone(),
                        project_addresses.destination_projects.projects.membrane.cdp.clone(),
                        position_id,
                    ),
                };

                // permission for whatever the follow-up action may be
                let other_grants = match ltv_ratio_threshold {
                    Some(RepayThreshold { otherwise, .. }) => gen_comp_pref_grants(GrantStructure {
                        granter: granter.clone(),
                        grantee: grantee.clone(),
                        expiration: expiration.clone(),
                        grant_contract: grant_contract.clone(),
                        grant_data: CompPrefsWithAddresses {
                            comp_prefs: OsmostakeCompoundPrefs {
                                comp_prefs: CompoundPrefs {
                                    relative: vec![DestinationAction {
                                        amount: 1u128.into(),
                                        destination: *otherwise,
                                    }],
                                },
                                user_address: user_address.clone(),
                                tax_fee: tax_fee.clone(),
                            },
                            project_addresses: project_addresses.clone(),
                            take_rate: take_rate.clone(),
                        },
                    })
                    // Normally we wouldn't unwrap but since this is solely the simulation query it should be alright
                    .unwrap(),
                    None => vec![],
                };

                [repay_grants, other_grants].concat()
            }
        }
    });

    Ok(dedupe_grant_reqs(grant_specs.collect()))
}
