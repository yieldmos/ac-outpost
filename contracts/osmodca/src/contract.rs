use crate::error::ContractError;
use crate::msg::{CompPrefsWithAddresses, ExecuteMsg, InstantiateMsg, MigrateMsg, OsmodcaCompoundPrefs, QueryMsg};
use crate::state::{
    ADMIN, AUTHORIZED_ADDRS, KNOWN_DENOMS, KNOWN_OSMO_POOLS, KNOWN_USDC_POOLS, PROJECT_ADDRS, TAKE_RATE, TWAP_DURATION,
};
use crate::{execute, queries};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult, Timestamp,
};
use cw2::{get_contract_version, set_contract_version};
use cw_grant_spec::grantable_trait::{GrantStructure, Grantable};
use osmosis_destinations::pools::PoolForEach;
use outpost_utils::comp_prefs::TakeRate;
use outpost_utils::helpers::CompoundingFrequency;
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ac-outpost-osmodca";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg {
        // an id of 0 means we don't care about the response
        Reply { id: 0, .. } => Ok(Response::default()),
        // TODO handle non-zero ids
        _ => Err(ContractError::Unauthorized {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, info: MessageInfo, msg: InstantiateMsg) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let InstantiateMsg {
        admin,
        project_addresses,
        max_tax_fee,
        take_rate_address,
        twap_duration,
    } = msg;

    let admin_addr = match admin {
        Some(admin) => deps
            .api
            .addr_validate(&admin)
            .map_err(|_| ContractError::InvalidAuthorizedAddress(admin.to_string()))?,
        None => info.sender,
    };

    ADMIN.save(deps.storage, &admin_addr)?;
    // Store the outpost take rate address
    TAKE_RATE.save(deps.storage, &TakeRate::new(deps.api, max_tax_fee, &take_rate_address)?)?;
    AUTHORIZED_ADDRS.save(deps.storage, &vec![])?;
    let validated_addrs = project_addresses.validate_addrs(deps.api)?;
    PROJECT_ADDRS.save(deps.storage, &validated_addrs)?;

    TWAP_DURATION.save(deps.storage, &twap_duration.u64())?;

    // store all the denoms in a map
    validated_addrs
        .destination_projects
        .denoms
        .denoms()
        .into_iter()
        .for_each(|(short_name, denom)| {
            let _ = KNOWN_DENOMS.save(deps.storage, denom, &short_name.to_string());
        });

    // store all the osmo pools in a map
    validated_addrs
        .destination_projects
        .swap_routes
        .osmo_pools
        .store_as_map(deps.storage, KNOWN_OSMO_POOLS);

    // store all the usdc pools in a map
    validated_addrs
        .destination_projects
        .swap_routes
        .usdc_pools
        .store_as_map(deps.storage, KNOWN_USDC_POOLS);

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    } else {
        return Err(ContractError::MigrationVersionMismatch {
            expected: storage_version.to_string(),
            received: version.to_string(),
        });
    }

    if let MigrateMsg {
        project_addresses: Some(addresses),
        max_tax_fee,
        take_rate_address,
    } = msg
    {
        let validated_addrs = addresses.validate_addrs(deps.api)?;

        PROJECT_ADDRS.save(deps.storage, &validated_addrs)?;

        // update the take rate
        TAKE_RATE.save(deps.storage, &TakeRate::new(deps.api, max_tax_fee, &take_rate_address)?)?;

        // clear the state that depends on the addresses data so we can reinitialize it
        KNOWN_DENOMS.clear(deps.storage);
        KNOWN_OSMO_POOLS.clear(deps.storage);
        KNOWN_USDC_POOLS.clear(deps.storage);

        // store all the denoms in a map
        validated_addrs
            .destination_projects
            .denoms
            .denoms()
            .into_iter()
            .for_each(|(short_name, denom)| {
                let _ = KNOWN_DENOMS.save(deps.storage, denom, &short_name.to_string());
            });

        // store all the osmo pools in a map
        validated_addrs
            .destination_projects
            .swap_routes
            .osmo_pools
            .store_as_map(deps.storage, KNOWN_OSMO_POOLS);

        // store all the usdc pools in a map
        validated_addrs
            .destination_projects
            .swap_routes
            .usdc_pools
            .store_as_map(deps.storage, KNOWN_USDC_POOLS);
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateProjectAddresses(addresses) => {
            if info.sender != ADMIN.load(deps.storage)? {
                return Err(ContractError::Unauthorized {});
            }
            PROJECT_ADDRS.save(deps.storage, &addresses.validate_addrs(deps.api)?)?;

            Ok(Response::default())
        }
        ExecuteMsg::AddAuthorizedCompounder(address) => {
            if info.sender != ADMIN.load(deps.storage)? {
                return Err(ContractError::Unauthorized {});
            }
            let authorized_addr = deps
                .api
                .addr_validate(&address)
                .map_err(|_| ContractError::InvalidAuthorizedAddress(address.to_string()))?;

            AUTHORIZED_ADDRS.update(deps.storage, |mut addrs| {
                if addrs.contains(&authorized_addr) {
                    Err(ContractError::DuplicateAuthorizedAddress(authorized_addr.to_string()))
                } else {
                    addrs.push(authorized_addr.clone());
                    Ok(addrs)
                }
            })?;

            Ok(Response::default())
        }
        ExecuteMsg::RemoveAuthorizedCompounder(address) => {
            if info.sender != ADMIN.load(deps.storage)? {
                return Err(ContractError::Unauthorized {});
            }
            let authorized_addr = deps
                .api
                .addr_validate(&address)
                .map_err(|_| ContractError::InvalidAuthorizedAddress(address.to_string()))?;

            AUTHORIZED_ADDRS.update(deps.storage, |mut addrs| -> Result<_, StdError> {
                addrs.retain(|x| x != authorized_addr);
                Ok(addrs)
            })?;

            Ok(Response::default())
        }
        ExecuteMsg::Compound(OsmodcaCompoundPrefs {
            user_address,
            comp_prefs,
            tax_fee: fee_to_charge,
        }) => {
            let addresses = PROJECT_ADDRS.load(deps.storage)?;
            let take_rate = TAKE_RATE.load(deps.storage)?;

            let prefs = comp_prefs.first().ok_or(ContractError::NoDCACompoundPrefs)?;
            if prefs.compound_token.denom != "uosmo" || (comp_prefs.len() > 1) {
                return Err(ContractError::InvalidDCACompoundPrefs);
            }

            execute::compound(deps, env, info, addresses, user_address, prefs, fee_to_charge, take_rate)
        }

        ExecuteMsg::ChangeTwapDuration(new_duration) => {
            if info.sender != ADMIN.load(deps.storage)? {
                return Err(ContractError::Unauthorized {});
            }
            TWAP_DURATION.save(deps.storage, &new_duration.u64())?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Version {} => to_json_binary(&queries::query_version()),
        QueryMsg::AuthorizedCompounders {} => to_json_binary(&queries::query_authorized_compounders(deps)),
        QueryMsg::TwapDuration => to_json_binary(&TWAP_DURATION.load(deps.storage)?),
        QueryMsg::GrantSpec {
            comp_prefs,
            frequency,
            expiration,
        } => {
            let project_addresses = PROJECT_ADDRS.load(deps.storage)?;
            let take_rate = TAKE_RATE.load(deps.storage)?;

            to_json_binary(&QueryMsg::query_grants(
                GrantStructure {
                    grantee: env.contract.address.clone(),
                    granter: deps.api.addr_validate(&comp_prefs.user_address)?,
                    expiration,
                    grant_contract: env.contract.address,
                    grant_data: CompPrefsWithAddresses {
                        comp_frequency: frequency,
                        comp_prefs,
                        project_addresses,
                        take_rate,
                    },
                },
                env.block.time,
            )?)
        }
        QueryMsg::RevokeSpec { comp_prefs } => {
            let project_addresses = PROJECT_ADDRS.load(deps.storage)?;
            let take_rate = TAKE_RATE.load(deps.storage)?;

            to_json_binary(&QueryMsg::query_revokes(GrantStructure {
                grantee: env.contract.address.clone(),
                granter: deps.api.addr_validate(&comp_prefs.user_address)?,
                expiration: Timestamp::default(),
                grant_contract: env.contract.address,
                grant_data: CompPrefsWithAddresses {
                    comp_frequency: CompoundingFrequency::default(),
                    comp_prefs,
                    project_addresses,
                    take_rate,
                },
            })?)
        }
    }
}
