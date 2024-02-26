#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::ValidStratId;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::queries::all_strat_ids;
use crate::state::{
    CompPref, EndType, InactiveStatus, StoreSettings, ALLOWED_STRATEGY_IDS, COMP_PREFS,
    PREFS_BY_PUBKEY, STORE_SETTINGS,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ymos-comp-prefs";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let InstantiateMsg {
        admin,
        chain_id,
        days_to_prune,
    } = msg;

    STORE_SETTINGS.save(
        deps.storage,
        &StoreSettings {
            // if no admin address is supplied we can just use the sender
            admin: admin.as_ref().map_or(Ok(info.sender), |admin| {
                deps.api
                    .addr_validate(admin)
                    .map_err(|_| ContractError::InvalidAdminAddress(admin.clone()))
            })?,
            chain_id,
            days_to_prune,
        },
    )?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
// #[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // ensure that the sender is the admin
    match msg {
        ExecuteMsg::AddAllowedStrategyId(_)
        | ExecuteMsg::RemoveAllowedStrategyId(_)
        | ExecuteMsg::SetAdmin(_) => {
            if STORE_SETTINGS.load(deps.storage)?.admin.ne(&info.sender) {
                return Err(ContractError::Unauthorized);
            }
        }
        _ => (),
    }

    match msg {
        ExecuteMsg::SetAdmin(admin) => {
            let mut settings = STORE_SETTINGS.load(deps.storage)?;
            settings.admin = deps
                .api
                .addr_validate(&admin)
                .map_err(|_| ContractError::InvalidAdminAddress(admin))?;
            STORE_SETTINGS.save(deps.storage, &settings)?;
            Ok(Response::new())
        }
        ExecuteMsg::AddAllowedStrategyId(strat_id) => {
            ALLOWED_STRATEGY_IDS.save(deps.storage, strat_id.u64(), &())?;
            Ok(Response::new().add_attribute("add allowed strategy id", strat_id))
        }
        ExecuteMsg::RemoveAllowedStrategyId(strat_id) => {
            ALLOWED_STRATEGY_IDS.remove(deps.storage, strat_id.u64());
            Ok(Response::new().add_attribute("remove allowed strategy id", strat_id))
        }
        ExecuteMsg::SetCompoundingPreferences(unverified_comp_pref) => {
            let settings = STORE_SETTINGS.load(deps.storage)?;

            // the address we're storing settings for
            let target_address = deps
                .api
                .addr_validate(&unverified_comp_pref.address)
                .map_err(|_| {
                    ContractError::InvalidUserAddress(unverified_comp_pref.address.clone())
                })?;

            // ensure that the target address is either the sender or admin
            if target_address.ne(&info.sender) && target_address.ne(&settings.admin) {
                return Err(ContractError::IncorrectCompPrefAddress);
            }

            // validate the compounding preferences
            let valid_comp_prefs =
                unverified_comp_pref.verify(deps.storage, deps.api, &env.block.time)?;

            // store the compounding preferences in state
            COMP_PREFS.update(
                deps.storage,
                (valid_comp_prefs.strat_id, &target_address),
                |prev_pref| -> Result<CompPref, ContractError> {
                    Ok(match prev_pref {
                        Some(prev_pref) => CompPref {
                            user_comp_pref: valid_comp_prefs.clone(),
                            chain_id: prev_pref.chain_id,
                            created_at: prev_pref.created_at,
                            updated_at: env.block.time,
                            is_inactive: None,
                        },
                        None => CompPref::new(
                            settings.chain_id,
                            valid_comp_prefs.clone(),
                            env.block.time,
                        ),
                    })
                },
            )?;

            // ensure that the pubkey path is also stored
            PREFS_BY_PUBKEY.save(
                deps.storage,
                (
                    &valid_comp_prefs.pub_key,
                    valid_comp_prefs.strat_id,
                    target_address.clone(),
                ),
                &(),
            )?;

            Ok(Response::new().add_attribute(
                "set compounding preferences",
                format!("{}-{}", valid_comp_prefs.strat_id, target_address),
            ))
        }
        ExecuteMsg::CancelCompoundingPreferences(strat_id) => {
            COMP_PREFS.update(
                deps.storage,
                (strat_id.u64(), &info.sender),
                |prev_pref| -> Result<CompPref, ContractError> {
                    match prev_pref {
                        Some(mut prev_pref) => {
                            prev_pref.is_inactive = Some(InactiveStatus {
                                end_type: EndType::Cancellation,
                                ended_at: env.block.time,
                            });

                            Ok(prev_pref)
                        }
                        None => Err(ContractError::NoSettingsFound(
                            info.sender.to_string(),
                            strat_id,
                        )),
                    }
                },
            )?;

            Ok(Response::new().add_attribute(
                "cancel compounding preferences",
                format!("{}-{}", strat_id, info.sender),
            ))
        }
        ExecuteMsg::PruneInactiveCompoundingPreferences { .. } => {
            if STORE_SETTINGS.load(deps.storage)?.admin.ne(&info.sender) {
                return Err(ContractError::Unauthorized);
            }

            Err(ContractError::Unimplemented)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
// #[cfg_attr(feature = "interface", cw_orch::interface_entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::StoreSettings => Ok(to_json_binary(&STORE_SETTINGS.load(deps.storage)?)?),
        QueryMsg::AllowedStrategyIds => Ok(to_json_binary(&all_strat_ids(deps.storage))?),
        QueryMsg::StrategyPreferencesByUserAndStratId {
            user_address,
            strategy_id,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;

            Ok(to_json_binary(&COMP_PREFS.may_load(
                deps.storage,
                (strategy_id.into(), &user_addr),
            )?)?)
        }
        QueryMsg::StrategyPreferencesByStratId {
            strat_id,
            status,
            limit,
            prev_address: _prev_address,
        } => {
            strat_id.valid_strat_id(deps.storage)?;

            let mut matching_prefs: Vec<CompPref> = vec![];

            // TODO: figure out how to do this bound
            // let range_min = prev_address
            //     .and_then(|prev_address| deps.api.addr_validate(&prev_address).ok())
            //     .map(|addr| cw_storage_plus::Bound::Exclusive((strat_id.u64(), addr)));

            let range_min = None;

            let iterable_prefs = COMP_PREFS.prefix(strat_id.u64()).range(
                deps.storage,
                range_min,
                None,
                Order::Ascending,
            );

            if status.is_some() {
                // if there's a status filter then we can use take wile to filter and take as we go
                let taken = iterable_prefs.take_while(|possible_pref| {
                    match possible_pref {
                        Ok((_, pref)) if pref.matches_status_filter(&status, &env.block.time) => {
                            deps.api.debug(&format!("matching pref: {:?}", pref));
                            matching_prefs.push(pref.clone());
                        }

                        _ => (),
                    };
                    limit.map_or(true, |limit| matching_prefs.len().lt(&limit.into()))
                });

                let _: Vec<Result<(Addr, CompPref), cosmwasm_std::StdError>> = taken.collect();
            } else {
                let results = if let Some(limit) = limit {
                    // if there's no status filter but a limit then we can just use take to grab the next limit number of results
                    Box::new(iterable_prefs.take(limit.into()))
                } else {
                    // otherwise just grab everything possible
                    iterable_prefs
                };

                matching_prefs = results
                    .filter_map(|possible_pref| possible_pref.map_or(None, |(_, pref)| Some(pref)))
                    .collect();
            }

            Ok(to_json_binary(&matching_prefs)?)
        }
        QueryMsg::StrategyPreferencesByUser {
            user_address,
            status,
        } => {
            // make sure we're searching for a realistic user address
            let user_addr = deps
                .api
                .addr_validate(&user_address)
                .map_err(|_| ContractError::InvalidUserAddress(user_address))?;

            let strat_ids = all_strat_ids(deps.storage);

            // look up all the relevant pairs of strategy ids with our search address in state
            let user_prefs: Vec<CompPref> = strat_ids
                .into_iter()
                .filter_map(|strat_id| -> Option<CompPref> {
                    match COMP_PREFS.may_load(deps.storage, (strat_id.u64(), &user_addr)) {
                        Ok(Some(pref)) if pref.matches_status_filter(&status, &env.block.time) => {
                            Some(pref)
                        }
                        _ => None,
                    }
                })
                .collect();

            Ok(to_json_binary(&user_prefs)?)
        }
        QueryMsg::StrategyPreferencesByPubkey { pubkey: _, status: _ } => {
            let user_prefs: Vec<CompPref> = vec![];
            // look up the pubkey in state
            // PREFS_BY_PUBKEY.prefix(&pubkey.as_str()).range(deps.storage, None, None, Order::Ascending).filter_map(|strat_keys|
            //     match strat_keys {
            //         Ok(( strat_id, user_addr)) => {
            //             // if the pubkey was in state we'll look up each entry from PREFS_BY_PUBKEY in COMP_PREFs
            //             match COMP_PREFS.may_load(deps.storage, (strat_id, user_addr)) {
            //                 Ok(Some(pref))
            //                 // ensure we're abiding by the status filtering
            //                     if pref.matches_status_filter(&status, &env.block.time) =>
            //                 {
            //                     Some(pref)
            //                 }
            //                 _ => None,
            //             }
            //         },
            //         _ => None
            //     }
            // ).collect();

            Ok(to_json_binary(&user_prefs)?)
        }
    }
}
